use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::types::LifecycleState;

/// Metadata stored in `snapshot.json` for each snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub name: String,
    pub created_at: String,
    pub forge_version: String,
    pub runtime_count: usize,
    pub description: Option<String>,
    pub state: LifecycleState,
}

/// Manages snapshot CRUD as directory-based flat files under `.forge/snapshots/{name}/`.
pub struct SnapshotManager {
    workspace_root: PathBuf,
    snapshots_dir: PathBuf,
    cache_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new SnapshotManager for the given workspace.
    ///
    /// The snapshots directory is `<workspace_root>/.forge/snapshots/`.
    pub fn new(workspace_root: &Path, cache_dir: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            snapshots_dir: workspace_root.join(".forge").join("snapshots"),
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Create a snapshot of the current environment state.
    ///
    /// Copies `forge.toml`, `forge.lock`, captures `state.json` via
    /// [`crate::state::compute_current_state`], captures the last 100
    /// journal lines from `.forge/journal.jsonl`, and writes a
    /// `snapshot.json` with metadata.
    ///
    /// Returns the snapshot name on success.
    pub fn create(&self, name: Option<&str>, description: Option<&str>) -> Result<String, String> {
        let snapshot_name = match name {
            Some(n) => n.to_string(),
            None => timestamp_name(),
        };

        let snapshot_dir = self.snapshots_dir.join(&snapshot_name);
        std::fs::create_dir_all(&snapshot_dir)
            .map_err(|e| format!("Failed to create snapshot directory '{}': {}", snapshot_dir.display(), e))?;

        // --- Validate forge.toml exists ---
        let toml_src = self.workspace_root.join("forge.toml");
        if !toml_src.exists() {
            let _ = std::fs::remove_dir_all(&snapshot_dir);
            return Err("forge.toml not found. Cannot create snapshot.".to_string());
        }

        // --- Copy forge.toml (verbatim) ---
        let toml_dst = snapshot_dir.join("forge.toml");
        std::fs::copy(&toml_src, &toml_dst)
            .map_err(|e| format!("Failed to copy forge.toml: {}", e))?;

        // --- Copy forge.lock (verbatim, if it exists) ---
        let lock_src = self.workspace_root.join("forge.lock");
        let lock_dst = snapshot_dir.join("forge.lock");
        if lock_src.exists() {
            std::fs::copy(&lock_src, &lock_dst)
                .map_err(|e| format!("Failed to copy forge.lock: {}", e))?;
        }

        // --- Capture lifecycle state via compute_current_state ---
        let state = crate::state::compute_current_state(&self.workspace_root, &self.cache_dir);
        let state_json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        std::fs::write(snapshot_dir.join("state.json"), &state_json)
            .map_err(|e| format!("Failed to write state.json: {}", e))?;

        // --- Capture last 100 lines of journal.jsonl ---
        let journal_path = self.workspace_root.join(".forge").join("journal.jsonl");
        let journal_dst = snapshot_dir.join("journal.jsonl");
        if journal_path.exists() {
            let content = std::fs::read_to_string(&journal_path)
                .map_err(|e| format!("Failed to read journal: {}", e))?;
            let lines: Vec<&str> = content.lines().collect();
            let last_100: Vec<&str> = if lines.len() > 100 {
                lines[lines.len() - 100..].to_vec()
            } else {
                lines
            };
            std::fs::write(&journal_dst, last_100.join("\n"))
                .map_err(|e| format!("Failed to write journal.jsonl: {}", e))?;
        }

        // --- Count runtimes from forge.toml ---
        let runtime_count = match crate::manifest::load_config(&toml_src) {
            Ok(config) => config.runtimes.len(),
            Err(_) => 0,
        };

        // --- Write snapshot.json ---
        let created_at = format_iso_timestamp();
        let metadata = SnapshotMetadata {
            name: snapshot_name.clone(),
            created_at,
            forge_version: env!("CARGO_PKG_VERSION").to_string(),
            runtime_count,
            description: description.map(|d| d.to_string()),
            state,
        };
        let meta_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Failed to serialize snapshot metadata: {}", e))?;
        std::fs::write(snapshot_dir.join("snapshot.json"), &meta_json)
            .map_err(|e| format!("Failed to write snapshot.json: {}", e))?;

        Ok(snapshot_name)
    }

    /// List all snapshots sorted by `created_at` descending.
    pub fn list(&self) -> Result<Vec<SnapshotMetadata>, String> {
        if !self.snapshots_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        let entries = std::fs::read_dir(&self.snapshots_dir)
            .map_err(|e| format!("Failed to read snapshots directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let meta_path = path.join("snapshot.json");
            if !meta_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&meta_path)
                .map_err(|e| format!("Failed to read {}: {}", meta_path.display(), e))?;
            match serde_json::from_str::<SnapshotMetadata>(&content) {
                Ok(meta) => snapshots.push(meta),
                Err(e) => eprintln!(
                    "Warning: Skipping snapshot '{}' due to parse error: {}",
                    path.display(),
                    e
                ),
            }
        }

        // Sort by created_at descending (most recent first), then by name ascending as tiebreaker
        snapshots.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(snapshots)
    }

    /// Restore files from a named snapshot.
    ///
    /// Backs up current `forge.toml` and `forge.lock` to `.bak` files,
    /// then copies the snapshot's versions into the workspace.
    ///
    /// On `dry_run`, no files are modified — only a preview is printed.
    pub fn restore(&self, name: &str, dry_run: bool) -> Result<(), String> {
        let snapshot_dir = self.snapshots_dir.join(name);
        if !snapshot_dir.exists() {
            return Err(format!("Snapshot '{}' not found", name));
        }

        let snapshot_toml = snapshot_dir.join("forge.toml");
        if !snapshot_toml.exists() {
            return Err(format!("Snapshot '{}' is missing forge.toml", name));
        }

        let snapshot_lock = snapshot_dir.join("forge.lock");
        let current_toml = self.workspace_root.join("forge.toml");
        let current_lock = self.workspace_root.join("forge.lock");

        if dry_run {
            println!("Would restore from snapshot '{}':", name);
            println!("  forge.toml -> {}", current_toml.display());
            if snapshot_lock.exists() {
                println!("  forge.lock -> {}", current_lock.display());
            }
            return Ok(());
        }

        // --- Backup current files ---
        let bak_toml = self.workspace_root.join("forge.toml.bak");
        let bak_lock = self.workspace_root.join("forge.lock.bak");

        if current_toml.exists() {
            std::fs::copy(&current_toml, &bak_toml)
                .map_err(|e| format!("Failed to backup forge.toml: {}", e))?;
        }
        if current_lock.exists() {
            std::fs::copy(&current_lock, &bak_lock)
                .map_err(|e| format!("Failed to backup forge.lock: {}", e))?;
        }

        // --- Restore from snapshot ---
        let restore_toml = || -> Result<(), String> {
            std::fs::copy(&snapshot_toml, &current_toml)
                .map_err(|e| format!("Failed to restore forge.toml: {}", e))?;

            if snapshot_lock.exists() {
                std::fs::copy(&snapshot_lock, &current_lock)
                    .map_err(|e| format!("Failed to restore forge.lock: {}", e))?;
            } else if current_lock.exists() {
                std::fs::remove_file(&current_lock)
                    .map_err(|e| format!("Failed to remove stale forge.lock: {}", e))?;
            }
            Ok(())
        };

        if let Err(e) = restore_toml() {
            // Rollback .bak files on failure
            let _ = std::fs::copy(&bak_toml, &current_toml);
            if bak_lock.exists() {
                let _ = std::fs::copy(&bak_lock, &current_lock);
            }
            return Err(e);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a UTC timestamp suitable as a directory name: `YYYY-MM-DDTHH-MM-SS`
fn timestamp_name() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (y, mo, day, h, m, s) = secs_to_ymdhms(d);
    format!("{:04}-{:02}-{:02}T{:02}-{:02}-{:02}", y, mo, day, h, m, s)
}

/// Generate an ISO 8601 UTC timestamp string: `YYYY-MM-DDTHH:MM:SSZ`
fn format_iso_timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (y, mo, day, h, m, s) = secs_to_ymdhms(d);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, h, m, s)
}

fn secs_to_ymdhms(total_secs: u64) -> (i64, i64, i64, i64, i64, i64) {
    let days = total_secs / 86400;
    let time_secs = total_secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    let (y, mo, day) = days_to_date(days as i64);
    (y, mo, day, h as i64, m as i64, s as i64)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_to_date(mut days: i64) -> (i64, i64, i64) {
    let mut y = 1970i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }

    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut mo = 1i64;
    for &md in month_days.iter() {
        if days < md {
            break;
        }
        days -= md;
        mo += 1;
    }
    let day = days + 1;

    (y, mo, day)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TestContext {
        _guard: PathBuf,
    }

    impl TestContext {
        fn new() -> (Self, PathBuf) {
            let count = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let base = std::env::temp_dir().join(format!(
                "forge_snapshot_test_{}_{}",
                std::process::id(),
                count
            ));
            let ws = base.join("ws");
            let _ = std::fs::remove_dir_all(&base);
            std::fs::create_dir_all(&ws).unwrap();
            let ctx = TestContext { _guard: base };
            (ctx, ws)
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self._guard);
        }
    }

    fn init_workspace(ws: &Path, toml_content: &str, lock_content: &str) {
        // Create .forge directory
        std::fs::create_dir_all(ws.join(".forge")).unwrap();
        std::fs::write(ws.join("forge.toml"), toml_content).unwrap();
        if !lock_content.is_empty() {
            std::fs::write(ws.join("forge.lock"), lock_content).unwrap();
        }
        // Write some journal events
        let journal_content: String = (0..50)
            .map(|i| format!("{{\"event\": {}, \"ts\": \"2025-01-01T00:00:00Z\"}}\n", i))
            .collect();
        std::fs::write(ws.join(".forge").join("journal.jsonl"), &journal_content).unwrap();
    }

    fn create_manager(ws: &Path) -> SnapshotManager {
        let cache_dir = ws.join(".forge").join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        SnapshotManager::new(ws, &cache_dir)
    }

    // -----------------------------------------------------------------------
    // 4.1: Create snapshot creates all expected files
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_snapshot_creates_all_files() {
        let (_ctx, ws) = TestContext::new();
        let toml_content = "[runtimes]\nnode = \">=18\"\npython = \">=3.11\"\n";
        let lock_content = "[node]\nversion = \"18.12.0\"\n";
        init_workspace(&ws, toml_content, lock_content);

        let manager = create_manager(&ws);
        let name = manager.create(Some("test-snap"), Some("Test snapshot")).unwrap();

        let snap_dir = ws.join(".forge").join("snapshots").join(&name);

        // Verify all 5 files exist
        assert!(snap_dir.join("forge.toml").exists(), "forge.toml should exist");
        assert!(snap_dir.join("forge.lock").exists(), "forge.lock should exist");
        assert!(snap_dir.join("state.json").exists(), "state.json should exist");
        assert!(snap_dir.join("journal.jsonl").exists(), "journal.jsonl should exist");
        assert!(snap_dir.join("snapshot.json").exists(), "snapshot.json should exist");

        // Verify forge.toml is byte-identical
        let original_toml = std::fs::read(ws.join("forge.toml")).unwrap();
        let snap_toml = std::fs::read(snap_dir.join("forge.toml")).unwrap();
        assert_eq!(original_toml, snap_toml, "forge.toml should be byte-identical");

        // Verify forge.lock is byte-identical
        let original_lock = std::fs::read(ws.join("forge.lock")).unwrap();
        let snap_lock = std::fs::read(snap_dir.join("forge.lock")).unwrap();
        assert_eq!(original_lock, snap_lock, "forge.lock should be byte-identical");
    }

    // -----------------------------------------------------------------------
    // 4.2: List snapshots returns correct metadata
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_snapshots_empty() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(&ws, "[runtimes]\n", "");
        let manager = create_manager(&ws);
        let snapshots = manager.list().unwrap();
        assert!(snapshots.is_empty(), "should be empty when no snapshots exist");
    }

    #[test]
    fn test_list_snapshots_multiple_sorted() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(&ws, "[runtimes]\nnode = \">=18\"\n", "[node]\nversion = \"18.0.0\"\n");
        let manager = create_manager(&ws);

        manager.create(Some("alpha"), Some("First snapshot")).unwrap();
        manager.create(Some("beta"), Some("Second snapshot")).unwrap();
        manager.create(Some("gamma"), Some("Third snapshot")).unwrap();

        let snapshots = manager.list().unwrap();

        assert_eq!(snapshots.len(), 3, "should have 3 snapshots");

        let names: Vec<&str> = snapshots.iter().map(|m| m.name.as_str()).collect();
        // Created_at timestamps may be identical (same second),
        // so tiebreaker is name ascending: alpha, beta, gamma
        assert!(names.contains(&"alpha"), "should contain alpha");
        assert!(names.contains(&"beta"), "should contain beta");
        assert!(names.contains(&"gamma"), "should contain gamma");

        // Verify metadata
        let alpha = snapshots.iter().find(|m| m.name == "alpha").unwrap();
        assert_eq!(alpha.runtime_count, 1);
        assert_eq!(alpha.description.as_deref(), Some("First snapshot"));
        assert_eq!(alpha.forge_version, env!("CARGO_PKG_VERSION"));
    }

    // -----------------------------------------------------------------------
    // 4.3: Restore snapshot replaces files correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_restore_dry_run_does_not_modify_files() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(
            &ws,
            "[runtimes]\nnode = \">=18\"\n",
            "[node]\nversion = \"18.0.0\"\n",
        );
        let manager = create_manager(&ws);

        manager.create(Some("pre-upgrade"), Some("Before upgrade")).unwrap();

        // Modify current files
        std::fs::write(ws.join("forge.toml"), "[runtimes]\nmodified = \"*\"\n").unwrap();
        std::fs::write(ws.join("forge.lock"), "[modified]\nversion = \"2.0\"\n").unwrap();

        // Dry-run should not modify files
        manager.restore("pre-upgrade", true).unwrap();

        // Files should still have modified content
        let toml_content = std::fs::read_to_string(ws.join("forge.toml")).unwrap();
        assert!(toml_content.contains("modified"), "forge.toml should NOT be restored in dry-run");
    }

    #[test]
    fn test_restore_replaces_files_correctly() {
        let (_ctx, ws) = TestContext::new();
        let original_toml = "[runtimes]\nnode = \">=18\"\n";
        let original_lock = "[node]\nversion = \"18.0.0\"\n";
        init_workspace(&ws, original_toml, original_lock);
        let manager = create_manager(&ws);

        manager.create(Some("pre-upgrade"), Some("Before upgrade")).unwrap();

        // Modify current files
        let modified_toml = "[runtimes]\nmodified = \"*\"\n";
        let modified_lock = "[modified]\nversion = \"2.0\"\n";
        std::fs::write(ws.join("forge.toml"), modified_toml).unwrap();
        std::fs::write(ws.join("forge.lock"), modified_lock).unwrap();

        // Restore
        manager.restore("pre-upgrade", false).unwrap();

        // Files should be back to original
        let restored_toml = std::fs::read_to_string(ws.join("forge.toml")).unwrap();
        let restored_lock = std::fs::read_to_string(ws.join("forge.lock")).unwrap();
        assert_eq!(restored_toml, original_toml, "forge.toml should be restored");
        assert_eq!(restored_lock, original_lock, "forge.lock should be restored");
    }

    #[test]
    fn test_restore_missing_snapshot_errors() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(&ws, "[runtimes]\n", "");
        let manager = create_manager(&ws);

        let result = manager.restore("nonexistent", false);
        assert!(result.is_err(), "restoring non-existent snapshot should error");
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_restore_without_forge_toml_in_snapshot() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(&ws, "[runtimes]\nnode = \">=18\"\n", "");
        let manager = create_manager(&ws);

        // Create a snapshot then remove its forge.toml to simulate corruption
        manager.create(Some("corrupted"), None).unwrap();
        std::fs::remove_file(
            ws.join(".forge")
                .join("snapshots")
                .join("corrupted")
                .join("forge.toml"),
        )
        .unwrap();

        let result = manager.restore("corrupted", false);
        assert!(result.is_err(), "restoring corrupted snapshot should error");
    }

    #[test]
    fn test_create_auto_name_format() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(&ws, "[runtimes]\nn = \"*\"\n", "");
        let manager = create_manager(&ws);

        // No name provided → auto-name with timestamp
        let name = manager.create(None, None).unwrap();
        // Format: YYYY-MM-DDTHH-MM-SS (20 chars)
        assert_eq!(name.len(), 19, "timestamp name should be 19 chars");
        assert_eq!(&name[4..5], "-", "5th char should be dash");
        assert_eq!(&name[7..8], "-", "8th char should be dash");
        assert_eq!(&name[10..11], "T", "11th char should be T");
    }

    #[test]
    fn test_create_snapshot_metadata_content() {
        let (_ctx, ws) = TestContext::new();
        init_workspace(
            &ws,
            "[runtimes]\nnode = \">=18\"\npython = \">=3.11\"\n",
            "[node]\nversion = \"18.0.0\"\n",
        );
        let manager = create_manager(&ws);

        let name = manager
            .create(Some("metadata-test"), Some("Test description"))
            .unwrap();

        let meta_path = ws
            .join(".forge")
            .join("snapshots")
            .join(&name)
            .join("snapshot.json");
        let content = std::fs::read_to_string(&meta_path).unwrap();
        let meta: SnapshotMetadata = serde_json::from_str(&content).unwrap();

        assert_eq!(meta.name, "metadata-test");
        assert_eq!(meta.description.as_deref(), Some("Test description"));
        assert_eq!(meta.runtime_count, 2);
        assert!(meta.created_at.ends_with("Z"), "timestamp should end with Z");
        assert_eq!(meta.forge_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_restore_bak_files_created() {
        let (_ctx, ws) = TestContext::new();
        let original_toml = "[runtimes]\nnode = \">=18\"\n";
        let original_lock = "[node]\nversion = \"18.0.0\"\n";
        init_workspace(&ws, original_toml, original_lock);
        let manager = create_manager(&ws);

        manager.create(Some("pre-upgrade"), None).unwrap();

        // Modify current files
        std::fs::write(ws.join("forge.toml"), "modified").unwrap();
        std::fs::write(ws.join("forge.lock"), "modified").unwrap();

        // Restore
        manager.restore("pre-upgrade", false).unwrap();

        // Bak files should exist and contain the modified content
        let bak_toml = std::fs::read_to_string(ws.join("forge.toml.bak")).unwrap();
        let bak_lock = std::fs::read_to_string(ws.join("forge.lock.bak")).unwrap();
        assert_eq!(bak_toml, "modified", "bak should contain pre-restore content");
        assert_eq!(bak_lock, "modified", "bak should contain pre-restore content");
    }
}
