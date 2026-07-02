use std::path::Path;
use forge_core::types::Event;

/// Filtered audit entry derived from journal events.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub runtime: String,
    pub version: String,
    pub operation: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub verified: String,
}

/// Read the operation journal and return filtered audit entries.
/// Filters events by phase: Sync, Download, Extract, Commit.
pub fn read_audit_log(workspace_root: &Path) -> Result<Vec<AuditEntry>, String> {
    let journal_path = workspace_root.join(".forge").join("journal.jsonl");

    if !journal_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&journal_path)
        .map_err(|e| format!("Failed to read journal: {}", e))?;

    let relevant_phases = ["Sync", "Download", "Extract", "Commit"];
    let mut entries: Vec<AuditEntry> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => {
                // Skip malformed lines silently
                continue;
            }
        };

        if !relevant_phases.contains(&event.phase.as_str()) {
            continue;
        }

        // Extract version from message if available, otherwise use "—"
        let version = event
            .message
            .as_ref()
            .and_then(|msg| extract_version_from_msg(msg))
            .unwrap_or_else(|| "—".to_string());

        entries.push(AuditEntry {
            timestamp: event.timestamp,
            runtime: event.runtime,
            version,
            operation: event.phase.clone(),
            url: event.download_url.unwrap_or_else(|| "—".to_string()),
            size_bytes: event.file_size.unwrap_or(0),
            sha256: event.sha256.unwrap_or_else(|| "—".to_string()),
            verified: match event.verified {
                Some(true) => "verified".to_string(),
                Some(false) => "mismatch".to_string(),
                None => "not verified".to_string(),
            },
        });
    }

    Ok(entries)
}

/// Print audit entries as a formatted table.
pub fn print_audit_table(entries: &[AuditEntry]) {
    if entries.is_empty() {
        println!("No operations recorded");
        return;
    }

    // Column widths
    println!(
        "{:<24} | {:<12} | {:<10} | {:<10} | {:<50} | {:<10} | {:<64} | {:<12}",
        "Timestamp", "Runtime", "Version", "Operation", "URL", "Size", "SHA-256", "Verified"
    );
    println!("{}", "-".repeat(200));

    for entry in entries {
        let size_str = if entry.size_bytes > 0 {
            format!("{}", entry.size_bytes)
        } else {
            "—".to_string()
        };

        println!(
            "{:<24} | {:<12} | {:<10} | {:<10} | {:<50} | {:<10} | {:<64} | {:<12}",
            entry.timestamp,
            entry.runtime,
            entry.version,
            entry.operation,
            truncate_str(&entry.url, 50),
            size_str,
            truncate_str(&entry.sha256, 64),
            entry.verified,
        );
    }
}

/// Print audit entries as a JSON array.
pub fn print_audit_json(entries: &[AuditEntry]) {
    let json = serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string());
    println!("{}", json);
}

/// Extract version string from a message like "Downloading node v20.11.0".
fn extract_version_from_msg(msg: &str) -> Option<String> {
    // Look for " v" followed by a version-like pattern
    msg.split_whitespace()
        .find(|word| word.starts_with('v') && word.len() > 1 && word[1..].chars().any(|c| c.is_ascii_digit()))
        .map(|v| v.trim_start_matches('v').to_string())
}

/// Truncate a string to a maximum length, appending "…" if truncated.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        operation_id: &str,
        runtime: &str,
        phase: &str,
        timestamp: &str,
        download_url: Option<&str>,
        file_size: Option<u64>,
        sha256: Option<&str>,
        verified: Option<bool>,
        message: Option<&str>,
    ) -> Event {
        Event {
            timestamp: timestamp.to_string(),
            operation_id: operation_id.to_string(),
            runtime: runtime.to_string(),
            phase: phase.to_string(),
            status: forge_core::types::EventStatus::Finished,
            message: message.map(|s| s.to_string()),
            download_url: download_url.map(|s| s.to_string()),
            file_size,
            sha256: sha256.map(|s| s.to_string()),
            verified,
        }
    }

    #[test]
    fn test_filter_relevant_phases() {
        let events = vec![
            make_event("op-1", "node", "Download", "2026-07-01T10:00:00Z", None, None, None, None, Some("Downloading node v20.11.0")),
            make_event("op-2", "node", "Extract", "2026-07-01T10:01:00Z", None, None, None, None, Some("Extracting node v20.11.0")),
            make_event("op-3", "node", "Commit", "2026-07-01T10:02:00Z", None, None, None, None, Some("Installed node v20.11.0")),
            make_event("op-4", "node", "Sync", "2026-07-01T10:03:00Z", None, None, None, None, Some("Syncing node v20.11.0")),
            make_event("op-5", "node", "Resolve", "2026-07-01T10:04:00Z", None, None, None, None, None),
        ];

        let temp_dir = std::env::temp_dir().join("forge_test_audit_filter");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir.join(".forge")).unwrap();

        let journal_path = temp_dir.join(".forge").join("journal.jsonl");
        {
            let mut file = std::fs::File::create(&journal_path).unwrap();
            for e in &events {
                let line = serde_json::to_string(e).unwrap();
                use std::io::Write;
                writeln!(file, "{}", line).unwrap();
            }
        }

        let entries = read_audit_log(&temp_dir).unwrap();
        assert_eq!(entries.len(), 4); // Resolve should be filtered out
        assert!(entries.iter().all(|e| matches!(e.operation.as_str(), "Sync" | "Download" | "Extract" | "Commit")));
    }

    #[test]
    fn test_empty_journal_returns_empty() {
        let temp_dir = std::env::temp_dir().join("forge_test_audit_empty");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let workspace_root = temp_dir; // no journal.jsonl present

        let entries = read_audit_log(&workspace_root).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_version_from_msg() {
        assert_eq!(extract_version_from_msg("Downloading node v20.11.0"), Some("20.11.0".to_string()));
        assert_eq!(extract_version_from_msg("Extracting python v3.12.0"), Some("3.12.0".to_string()));
        assert_eq!(extract_version_from_msg("No version here"), None);
        assert_eq!(extract_version_from_msg("v"), None);
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello w…");
    }

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            timestamp: "2026-07-01T10:00:00Z".to_string(),
            runtime: "node".to_string(),
            version: "20.11.0".to_string(),
            operation: "Download".to_string(),
            url: "https://nodejs.org/dist/v20.11.0/node.zip".to_string(),
            size_bytes: 1234567,
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            verified: "verified".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
        assert!(json.contains("verified"));
    }
}
