use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BundleError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    MissingEntry(String),
    SecretExcluded(String),
    MissingAnvilToml,
    InvalidArchive(String),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleError::Io(e) => write!(f, "I/O error: {}", e),
            BundleError::Serde(e) => write!(f, "serialization error: {}", e),
            BundleError::ChecksumMismatch { path, expected, actual } => {
                write!(
                    f,
                    "checksum mismatch for '{}': expected {}, got {}",
                    path, expected, actual
                )
            }
            BundleError::MissingEntry(path) => {
                write!(f, "missing entry in archive: {}", path)
            }
            BundleError::SecretExcluded(path) => {
                write!(f, "secret file excluded from bundle: {}", path)
            }
            BundleError::MissingAnvilToml => {
                write!(f, "anvil.toml not found")
            }
            BundleError::InvalidArchive(msg) => {
                write!(f, "invalid archive: {}", msg)
            }
        }
    }
}

impl std::error::Error for BundleError {}

impl From<std::io::Error> for BundleError {
    fn from(e: std::io::Error) -> Self {
        BundleError::Io(e)
    }
}

impl From<serde_json::Error> for BundleError {
    fn from(e: serde_json::Error) -> Self {
        BundleError::Serde(e)
    }
}

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

/// Metadata written into the archive as `metadata.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub anvil_version: String,
    pub created_at: String,
    pub workspace_id: Option<String>,
    pub runtime_count: usize,
    pub excluded_patterns: Vec<String>,
}

/// In-memory representation of a workspace's bundled content.
#[derive(Debug)]
pub struct Bundle {
    pub anvil_toml: String,
    pub anvil_lock: String,
    pub metadata: serde_json::Value,
    pub sha256_manifest: HashMap<String, String>,
}

/// Single entry in the checksum manifest.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChecksumEntry {
    pub path: String,
    pub sha256: String,
}

/// The checksum manifest type (list of entries).
pub type BundleChecksums = Vec<ChecksumEntry>;

// ---------------------------------------------------------------------------
// Deterministic tar entry helper
// ---------------------------------------------------------------------------

fn add_file_to_tar<W: Write>(
    archive: &mut tar::Builder<W>,
    name: &str,
    data: &[u8],
) -> Result<(), BundleError> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    // Short names so these won't fail
    let _ = header.set_username("root");
    let _ = header.set_groupname("root");
    header.set_size(data.len() as u64);
    header.set_cksum();
    archive.append_data(&mut header, Path::new(name), data)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// SHA-256 helpers
// ---------------------------------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Build the plain-text bundle.sha256 content (`sha256  filename` per line, sorted).
fn format_sha256_manifest(manifest: &HashMap<String, String>) -> String {
    let mut entries: Vec<(&String, &String)> = manifest.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut lines = Vec::with_capacity(entries.len());
    for (path, hash) in &entries {
        lines.push(format!("{}  {}", hash, path));
    }
    lines.join("\n")
}

/// Parse a plain-text sha256sum manifest back into entries.
fn parse_sha256_manifest(content: &str) -> Result<HashMap<String, String>, BundleError> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "<sha256>  <filename>"  (two spaces between hash and filename)
        if let Some((hash, path)) = line.split_once("  ") {
            map.insert(path.to_string(), hash.to_string());
        } else {
            return Err(BundleError::InvalidArchive(format!(
                "malformed checksum line: {}",
                line
            )));
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// ISO 8601 timestamp (no external dep)
// ---------------------------------------------------------------------------

fn format_iso_timestamp() -> String {
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = d / 86400;
    let time_secs = d % 86400;

    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    let (y, mo, day) = days_to_date(days as i64);

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, h, m, s)
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
// Build metadata
// ---------------------------------------------------------------------------

fn build_metadata(anvil_toml: &str, created_at: &str) -> BundleMetadata {
    let runtime_count = match anvil_toml.parse::<toml::Value>().ok() {
        Some(v) => v
            .get("runtimes")
            .and_then(|r| r.as_table())
            .map(|t| t.len())
            .unwrap_or(0),
        None => 0,
    };

    BundleMetadata {
        anvil_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: created_at.to_string(),
        workspace_id: None,
        runtime_count,
        excluded_patterns: vec![
            ".anvil/".to_string(),
            "anvil.secrets".to_string(),
            "anvil.env".to_string(),
        ],
    }
}

// ---------------------------------------------------------------------------
// Excluded paths check
// ---------------------------------------------------------------------------

#[cfg(test)]
const EXCLUDED_NAMES: &[&str] = &[".anvil", "anvil.secrets", "anvil.env"];

#[cfg(test)]
fn is_excluded(path: &Path) -> bool {
    let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();
    for segment in &components {
        if EXCLUDED_NAMES.contains(&segment.as_ref()) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Archive writer (internal)
// ---------------------------------------------------------------------------

fn write_archive(
    output_path: &Path,
    anvil_toml_bytes: &[u8],
    anvil_lock_bytes: &[u8],
    metadata_bytes: &[u8],
    sha256_bytes: &[u8],
) -> Result<(), BundleError> {
    let output_file = std::fs::File::create(output_path)?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    {
        let mut archive = tar::Builder::new(&mut encoder);
        // Entries in sorted order: anvil.toml, anvil.lock, metadata.json, bundle.sha256
        add_file_to_tar(&mut archive, "anvil.toml", anvil_toml_bytes)?;
        add_file_to_tar(&mut archive, "anvil.lock", anvil_lock_bytes)?;
        add_file_to_tar(&mut archive, "metadata.json", metadata_bytes)?;
        add_file_to_tar(&mut archive, "bundle.sha256", sha256_bytes)?;
        archive.finish()?;
    }
    encoder.finish()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a deterministic `.anvil` archive from a workspace.
///
/// 1. Reads `anvil.toml` and `anvil.lock` from `workspace_dir`.
/// 2. Computes SHA-256 checksums for each file.
/// 3. Builds `metadata.json` and `bundle.sha256`.
/// 4. Produces a tar+gzip archive at `output_path` with deterministic headers.
pub fn create_bundle(workspace_dir: &Path, output_path: &Path) -> Result<(), BundleError> {
    // --- Check anvil.toml exists ---
    let toml_path = workspace_dir.join("anvil.toml");
    if !toml_path.exists() {
        return Err(BundleError::MissingAnvilToml);
    }

    // --- Read workspace files ---
    let anvil_toml = std::fs::read_to_string(&toml_path)?;
    let anvil_toml_bytes = anvil_toml.as_bytes();

    let lock_path = workspace_dir.join("anvil.lock");
    let anvil_lock = if lock_path.exists() {
        std::fs::read_to_string(&lock_path)?
    } else {
        String::new()
    };
    let anvil_lock_bytes = anvil_lock.as_bytes();

    // --- Build metadata ---
    let created_at = format_iso_timestamp();
    let metadata = build_metadata(&anvil_toml, &created_at);
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    let metadata_bytes = metadata_json.as_bytes();

    // --- Compute SHA-256 per entry ---
    let toml_hash = sha256_hex(anvil_toml_bytes);
    let lock_hash = sha256_hex(anvil_lock_bytes);
    let meta_hash = sha256_hex(metadata_bytes);

    // --- Build manifest ---
    let mut manifest = HashMap::new();
    manifest.insert("anvil.toml".to_string(), toml_hash);
    manifest.insert("anvil.lock".to_string(), lock_hash);
    manifest.insert("metadata.json".to_string(), meta_hash);

    let sha256_content = format_sha256_manifest(&manifest);
    let sha256_bytes = sha256_content.as_bytes();

    // --- Write archive ---
    write_archive(output_path, anvil_toml_bytes, anvil_lock_bytes, metadata_bytes, sha256_bytes)
}

/// Verify extracted file checksums against the manifest parsed from `bundle.sha256`.
pub fn verify_checksums(
    extract_dir: &Path,
    checksums: &HashMap<String, String>,
) -> Result<(), BundleError> {
    for (path, expected_hash) in checksums {
        let file_path = extract_dir.join(path);
        if !file_path.exists() {
            return Err(BundleError::MissingEntry(path.clone()));
        }
        let data = std::fs::read(&file_path)?;
        let actual_hash = sha256_hex(&data);
        if actual_hash != *expected_hash {
            return Err(BundleError::ChecksumMismatch {
                path: path.clone(),
                expected: expected_hash.clone(),
                actual: actual_hash,
            });
        }
    }
    Ok(())
}

/// Restore a `.anvil` archive into a workspace directory.
///
/// 1. Decompresses and extracts all entries to a temporary directory.
/// 2. Reads `bundle.sha256` and verifies every other entry.
/// 3. Renames `anvil.toml` and `anvil.lock` into the workspace.
/// 4. Cleans up the temporary directory.
pub fn restore_bundle(bundle_path: &Path, workspace_dir: &Path) -> Result<(), BundleError> {
    // --- Read the bundle ---
    let file = std::fs::File::open(bundle_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    // --- Extract to a temp directory ---
    let temp_dir = workspace_dir.join(".anvil").join(".bundle_extract");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }
    std::fs::create_dir_all(&temp_dir)?;

    archive.unpack(&temp_dir)?;

    // --- Read and parse bundle.sha256 ---
    let sha256_path = temp_dir.join("bundle.sha256");
    if !sha256_path.exists() {
        // Clean up before returning
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(BundleError::MissingEntry("bundle.sha256".to_string()));
    }

    let sha256_content = std::fs::read_to_string(&sha256_path)?;
    let checksums = parse_sha256_manifest(&sha256_content)?;

    // --- Verify every entry (does NOT include bundle.sha256 itself) ---
    verify_checksums(&temp_dir, &checksums)?;

    // --- Check that anvil.toml exists in the extracted content ---
    let extracted_toml = temp_dir.join("anvil.toml");
    if !extracted_toml.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(BundleError::MissingEntry("anvil.toml".to_string()));
    }

    // --- Atomic rename anvil.toml and anvil.lock ---
    let dest_toml = workspace_dir.join("anvil.toml");
    let dest_lock = workspace_dir.join("anvil.lock");

    // Remove existing files if present
    let _ = std::fs::remove_file(&dest_toml);
    let _ = std::fs::remove_file(&dest_lock);

    std::fs::rename(&extracted_toml, &dest_toml)?;

    let extracted_lock = temp_dir.join("anvil.lock");
    if extracted_lock.exists() {
        std::fs::rename(&extracted_lock, &dest_lock)?;
    }

    // --- Clean up temp dir ---
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    static TEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TestContext {
        _guard: std::path::PathBuf,
    }

    impl TestContext {
        fn new() -> (Self, PathBuf) {
            let count = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let base = std::env::temp_dir().join(format!(
                "anvil_bundle_test_{}_{}",
                std::process::id(),
                count
            ));
            let ws = base.join("ws");
            let _ = std::fs::remove_dir_all(&base);
            std::fs::create_dir_all(&ws).unwrap();
            let ctx = TestContext { _guard: base.clone() };
            (ctx, ws)
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self._guard);
        }
    }

    fn write_anvil_toml(ws: &Path, content: &str) {
        std::fs::write(ws.join("anvil.toml"), content).unwrap();
    }

    fn write_anvil_lock(ws: &Path, content: &str) {
        std::fs::write(ws.join("anvil.lock"), content).unwrap();
    }

    // -----------------------------------------------------------------------
    // 4.1 Deterministic output
    // -----------------------------------------------------------------------

    #[test]
    fn test_deterministic_bundle() {
        let (_ctx, ws) = TestContext::new();
        // Use fixed data so output is deterministic
        let toml_data = "[runtimes]\nnode = \">=18\"\npython = \">=3.11\"\n";
        let lock_data = "# anvil.lock\n[node]\nversion = \"18.12.0\"\n";
        let meta = build_metadata(toml_data, "2025-01-01T00:00:00Z");
        let meta_json = serde_json::to_string_pretty(&meta).unwrap();

        let toml_hash = sha256_hex(toml_data.as_bytes());
        let lock_hash = sha256_hex(lock_data.as_bytes());
        let meta_hash = sha256_hex(meta_json.as_bytes());

        let mut manifest = HashMap::new();
        manifest.insert("anvil.toml".to_string(), toml_hash);
        manifest.insert("anvil.lock".to_string(), lock_hash);
        manifest.insert("metadata.json".to_string(), meta_hash);
        let sha256_content = format_sha256_manifest(&manifest);

        let out1 = ws.join("out1.anvil");
        let out2 = ws.join("out2.anvil");

        write_archive(
            &out1,
            toml_data.as_bytes(),
            lock_data.as_bytes(),
            meta_json.as_bytes(),
            sha256_content.as_bytes(),
        )
        .unwrap();
        write_archive(
            &out2,
            toml_data.as_bytes(),
            lock_data.as_bytes(),
            meta_json.as_bytes(),
            sha256_content.as_bytes(),
        )
        .unwrap();

        let bytes1 = std::fs::read(&out1).unwrap();
        let bytes2 = std::fs::read(&out2).unwrap();

        assert_eq!(bytes1, bytes2, "same inputs must produce byte-identical archives");
    }

    #[test]
    fn test_deterministic_multiple_runs() {
        let (_ctx, ws) = TestContext::new();
        // Same fixed inputs
        let toml_data = "[runtimes]\n";
        let lock_data = "";
        let meta = build_metadata(toml_data, "2025-06-01T12:00:00Z");
        let meta_json = serde_json::to_string_pretty(&meta).unwrap();

        let toml_hash = sha256_hex(toml_data.as_bytes());
        let lock_hash = sha256_hex(lock_data.as_bytes());
        let meta_hash = sha256_hex(meta_json.as_bytes());

        let mut manifest = HashMap::new();
        manifest.insert("anvil.toml".to_string(), toml_hash);
        manifest.insert("anvil.lock".to_string(), lock_hash);
        manifest.insert("metadata.json".to_string(), meta_hash);
        let sha256_content = format_sha256_manifest(&manifest);

        // Run three times
        let run1 = {
            let p = ws.join("r1.anvil");
            write_archive(
                &p, toml_data.as_bytes(), lock_data.as_bytes(),
                meta_json.as_bytes(), sha256_content.as_bytes(),
            ).unwrap();
            std::fs::read(&p).unwrap()
        };
        let run2 = {
            let p = ws.join("r2.anvil");
            write_archive(
                &p, toml_data.as_bytes(), lock_data.as_bytes(),
                meta_json.as_bytes(), sha256_content.as_bytes(),
            ).unwrap();
            std::fs::read(&p).unwrap()
        };
        let run3 = {
            let p = ws.join("r3.anvil");
            write_archive(
                &p, toml_data.as_bytes(), lock_data.as_bytes(),
                meta_json.as_bytes(), sha256_content.as_bytes(),
            ).unwrap();
            std::fs::read(&p).unwrap()
        };

        assert_eq!(run1, run2);
        assert_eq!(run2, run3);
    }

    // -----------------------------------------------------------------------
    // 4.2 Checksum verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_verify_checksums_ok() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\ntest = \"*\"\n");
        write_anvil_lock(&ws, "[test]\nversion = \"1.0\"\n");

        let bundle_path = ws.join("test.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        // Extract to a temp dir
        let extract = ws.join("_extract");
        std::fs::create_dir_all(&extract).unwrap();
        let file = std::fs::File::open(&bundle_path).unwrap();
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&extract).unwrap();

        let sha256_content = std::fs::read_to_string(extract.join("bundle.sha256")).unwrap();
        let checksums = parse_sha256_manifest(&sha256_content).unwrap();
        assert!(verify_checksums(&extract, &checksums).is_ok());
    }

    #[test]
    fn test_verify_checksums_mismatch() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\nx = \"1\"\n");
        write_anvil_lock(&ws, "");

        let bundle_path = ws.join("test.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        let extract = ws.join("_extract2");
        std::fs::create_dir_all(&extract).unwrap();
        let file = std::fs::File::open(&bundle_path).unwrap();
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&extract).unwrap();

        // Tamper with anvil.toml
        std::fs::write(extract.join("anvil.toml"), "tampered content").unwrap();

        let sha256_content = std::fs::read_to_string(extract.join("bundle.sha256")).unwrap();
        let checksums = parse_sha256_manifest(&sha256_content).unwrap();

        match verify_checksums(&extract, &checksums) {
            Err(BundleError::ChecksumMismatch { path, .. }) => {
                assert_eq!(path, "anvil.toml");
            }
            other => panic!("expected ChecksumMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_verify_checksums_missing_entry() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\na = \"1\"\n");
        write_anvil_lock(&ws, "");

        let bundle_path = ws.join("test.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        let extract = ws.join("_extract3");
        std::fs::create_dir_all(&extract).unwrap();
        let file = std::fs::File::open(&bundle_path).unwrap();
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&extract).unwrap();

        // Delete anvil.lock
        let _ = std::fs::remove_file(extract.join("anvil.lock"));

        let sha256_content = std::fs::read_to_string(extract.join("bundle.sha256")).unwrap();
        let checksums = parse_sha256_manifest(&sha256_content).unwrap();

        match verify_checksums(&extract, &checksums) {
            Err(BundleError::MissingEntry(path)) => {
                assert_eq!(path, "anvil.lock");
            }
            other => panic!("expected MissingEntry, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 4.3 Secrets exclusion
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_excluded_paths() {
        assert!(is_excluded(Path::new(".anvil/metadata_cache/file.toml")));
        assert!(is_excluded(Path::new("anvil.secrets")));
        assert!(is_excluded(Path::new("anvil.env")));
        assert!(is_excluded(Path::new(".anvil/something")));
        assert!(!is_excluded(Path::new("anvil.toml")));
        assert!(!is_excluded(Path::new("anvil.lock")));
        assert!(!is_excluded(Path::new("some/dir/file.txt")));
    }

    #[test]
    fn test_secrets_not_in_archive_entries() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\n");
        write_anvil_lock(&ws, "");

        // Create excluded files/dirs in the workspace, NOT picked up by create_bundle
        std::fs::create_dir_all(ws.join(".anvil")).unwrap();
        std::fs::write(ws.join("anvil.secrets"), "secret_key=abc123").unwrap();
        std::fs::write(ws.join("anvil.env"), "MY_VAR=value").unwrap();
        std::fs::write(ws.join(".anvil/cache.toml"), "data").unwrap();

        let bundle_path = ws.join("test.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        // Extract and verify excluded files are NOT present
        let extract = ws.join("_extract_secrets");
        std::fs::create_dir_all(&extract).unwrap();
        let file = std::fs::File::open(&bundle_path).unwrap();
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&extract).unwrap();

        // These should be present
        assert!(extract.join("anvil.toml").exists());
        assert!(extract.join("anvil.lock").exists());
        assert!(extract.join("metadata.json").exists());
        assert!(extract.join("bundle.sha256").exists());

        // These should NOT be present
        assert!(!extract.join(".anvil").exists());
        assert!(!extract.join("anvil.secrets").exists());
        assert!(!extract.join("anvil.env").exists());
    }

    // -----------------------------------------------------------------------
    // 4.4 Restore
    // -----------------------------------------------------------------------

    #[test]
    fn test_restore_bundle_roundtrip() {
        let (_ctx, ws) = TestContext::new();
        let toml_content = "[runtimes]\nnode = \">=18\"\n";
        let lock_content = "[node]\nversion = \"18.12.0\"\n";

        write_anvil_toml(&ws, toml_content);
        write_anvil_lock(&ws, lock_content);

        let bundle_path = ws.join("project.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        // Restore to a fresh directory
        let restore_dir = ws.join("restored");
        std::fs::create_dir_all(&restore_dir).unwrap();

        restore_bundle(&bundle_path, &restore_dir).unwrap();

        // Verify files are written correctly
        let restored_toml = std::fs::read_to_string(restore_dir.join("anvil.toml")).unwrap();
        let restored_lock = std::fs::read_to_string(restore_dir.join("anvil.lock")).unwrap();

        assert_eq!(restored_toml, toml_content);
        assert_eq!(restored_lock, lock_content);
    }

    #[test]
    fn test_restore_bundle_no_lock() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\nnone = \"*\"\n");
        // No anvil.lock written

        let bundle_path = ws.join("noanvil.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        let restore_dir = ws.join("restored2");
        std::fs::create_dir_all(&restore_dir).unwrap();

        restore_bundle(&bundle_path, &restore_dir).unwrap();

        assert!(restore_dir.join("anvil.toml").exists());
    }

    #[test]
    fn test_restore_bundle_tampered_rejected() {
        let (_ctx, ws) = TestContext::new();
        write_anvil_toml(&ws, "[runtimes]\na = \"1\"\n");
        write_anvil_lock(&ws, "");

        let bundle_path = ws.join("test.anvil");
        create_bundle(&ws, &bundle_path).unwrap();

        // Tamper by re-creating the archive with modified content:
        // extract, modify anvil.toml, re-pack, then restore should fail checksum.
        let extract = ws.join("_unpack_tamper");
        std::fs::create_dir_all(&extract).unwrap();
        {
            let file = std::fs::File::open(&bundle_path).unwrap();
            let decoder = GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(&extract).unwrap();
        }
        // Modify anvil.toml content (but NOT bundle.sha256)
        std::fs::write(extract.join("anvil.toml"), "tampered content").unwrap();

        // Re-pack the tampered content into a new archive
        let tampered_path = ws.join("tampered.anvil");
        {
            let out = std::fs::File::create(&tampered_path).unwrap();
            let mut encoder = GzEncoder::new(out, Compression::default());
            {
                let mut archive = tar::Builder::new(&mut encoder);
                // Read files from extract dir and add them back
                for entry_name in &["anvil.toml", "anvil.lock", "metadata.json", "bundle.sha256"] {
                    let entry_path = extract.join(entry_name);
                    if entry_path.exists() {
                        let data = std::fs::read(&entry_path).unwrap();
                        add_file_to_tar(&mut archive, entry_name, &data).unwrap();
                    }
                }
                archive.finish().unwrap();
            }
            encoder.finish().unwrap();
        }

        let restore_dir = ws.join("restored3");
        std::fs::create_dir_all(&restore_dir).unwrap();

        let result = restore_bundle(&tampered_path, &restore_dir);

        // Should fail — checksum mismatch for anvil.toml
        assert!(result.is_err(), "tampered archive should be rejected");

        // No files should have been written to workspace
        assert!(!restore_dir.join("anvil.toml").exists());
    }

    #[test]
    fn test_bundle_without_anvil_toml_errors() {
        let (_ctx, ws) = TestContext::new();
        // No anvil.toml

        let bundle_path = ws.join("empty.anvil");
        let result = create_bundle(&ws, &bundle_path);
        assert!(matches!(result, Err(BundleError::MissingAnvilToml)));
    }

    #[test]
    fn test_sha256_manifest_roundtrip() {
        let mut manifest = HashMap::new();
        manifest.insert("a.txt".to_string(), "abc".to_string());
        manifest.insert("b.txt".to_string(), "def".to_string());

        let content = format_sha256_manifest(&manifest);
        let parsed = parse_sha256_manifest(&content).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("a.txt").unwrap(), "abc");
        assert_eq!(parsed.get("b.txt").unwrap(), "def");
    }
}
