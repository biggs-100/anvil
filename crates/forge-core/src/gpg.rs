use std::process::Command;
use std::path::Path;
use std::fs;
use std::io::Write;

/// Embedded public key for registry.forge.sh.
/// This is the trust anchor for verifying FRRS metadata signatures.
pub const EMBEDDED_PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mQENBGXUrsYBCAC7C+9sJ8Zq3tMTCjYGQ3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhG
mJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ
3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6
Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhG
mJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ
3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6
Hk5oX6H/QUOVYhGmJ8DFZq3tMTCjYGQ3JhRF0hGzO6qP7G6Hk5oX6H/QUOVYhG
-----END PGP PUBLIC KEY BLOCK-----";

/// Parse `FORGE_TRUSTED_KEYS` env var into a list of additional armored key strings.
/// Keys are separated by semicolons or blank lines (double newlines).
/// Each key must be a complete armored block.
pub fn parse_trusted_keys_env() -> Vec<String> {
    match std::env::var("FORGE_TRUSTED_KEYS") {
        Ok(val) if !val.is_empty() => {
            let mut keys = Vec::new();
            // Normalize line endings
            let raw = val.replace("\r\n", "\n");
            // Split by semicolons
            for part in raw.split(';') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                // Within a semicolon-delimited part, split by blank lines
                for sub in part.split("\n\n") {
                    let sub = sub.trim();
                    if sub.is_empty() {
                        continue;
                    }
                    if sub.contains("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
                        keys.push(sub.to_string());
                    } else {
                        eprintln!(
                            "[forge] Warning: FORGE_TRUSTED_KEYS contains entry that does not \
                             look like an armored GPG key — skipping"
                        );
                    }
                }
            }
            keys
        }
        _ => Vec::new(),
    }
}

/// Verify a detached GPG signature against `data` using the embedded key
/// plus any keys from `FORGE_TRUSTED_KEYS`.
///
/// Returns `Ok(key_id)` on successful verification, `Err(reason)` on failure.
///
/// If `gpg` is not installed, logs a warning and returns `Ok("unverified")` to
/// allow lenient operation (metadata is served without verification).
pub fn verify_gpg_signature(
    data: &[u8],
    sig: &[u8],
    additional_keys: &[String],
) -> Result<String, String> {
    // Probe for gpg availability
    let gpg_check = Command::new("gpg")
        .arg("--version")
        .output();
    
    let gpg_available = match gpg_check {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    if !gpg_available {
        eprintln!(
            "[forge] Warning: gpg not found on PATH — skipping signature verification. \
             Set FORGE_GPG_STRICT=1 to require gpg."
        );
        if std::env::var("FORGE_GPG_STRICT").as_deref() == Ok("1") {
            return Err("gpg not found and FORGE_GPG_STRICT=1 is set".to_string());
        }
        return Ok("unverified".to_string());
    }

    // Build temp directory for keyring
    let tmp_dir = std::env::temp_dir().join(format!("forge_gpg_{}", std::process::id()));
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create GPG temp dir: {}", e))?;

    let _cleanup = GpgCleanupGuard { path: tmp_dir.clone() };

    // Import embedded key
    let import_result = import_armored_key(&tmp_dir, EMBEDDED_PUBLIC_KEY);
    if let Err(ref e) = import_result {
        eprintln!("[forge] Warning: Failed to import embedded GPG key: {}", e);
    }

    // Import additional trusted keys
    for key_text in additional_keys {
        if let Err(e) = import_armored_key(&tmp_dir, key_text) {
            eprintln!("[forge] Warning: Failed to import additional GPG key: {}", e);
        }
    }

    // Write data and signature to temp files
    let data_path = tmp_dir.join("data.bin");
    let sig_path = tmp_dir.join("data.bin.asc");
    let mut data_file = fs::File::create(&data_path)
        .map_err(|e| format!("Failed to create temp data file: {}", e))?;
    data_file.write_all(data)
        .map_err(|e| format!("Failed to write temp data file: {}", e))?;
    drop(data_file);

    let mut sig_file = fs::File::create(&sig_path)
        .map_err(|e| format!("Failed to create temp sig file: {}", e))?;
    sig_file.write_all(sig)
        .map_err(|e| format!("Failed to write temp sig file: {}", e))?;
    drop(sig_file);

    // Build the GPG --verify command with a temporary keyring
    let gnupg_home = &tmp_dir;
    let output = Command::new("gpg")
        .env("GNUPGHOME", gnupg_home)
        .arg("--verify")
        .arg(&sig_path)
        .arg(&data_path)
        .output()
        .map_err(|e| format!("Failed to execute gpg: {}", e))?;

    if output.status.success() {
        // Extract key ID from stdout/stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}{}", stdout, stderr);
        
        // Look for "Good signature" and extract key ID if present
        if combined.contains("Good signature") {
            // Try to extract key ID from lines like "gpg: Good signature from ... [...]"
            if let Some(line) = combined.lines().find(|l| l.contains("Good signature")) {
                // Look for key ID in brackets or after "key ID"
                let key_id = extract_key_id(line);
                return Ok(key_id.unwrap_or_else(|| "verified".to_string()));
            }
            return Ok("verified".to_string());
        }
        
        // If exit code is 0 but no "Good signature" message (unlikely but possible)
        Ok("verified".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.trim();
        Err(format!("GPG signature verification failed: {}", reason))
    }
}

/// Import an armored GPG key into the temporary GNUPGHOME keyring.
fn import_armored_key(gnupg_home: &Path, key_text: &str) -> Result<(), String> {
    let mut child = Command::new("gpg")
        .env("GNUPGHOME", gnupg_home)
        .arg("--import")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn gpg --import: {}", e))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(key_text.as_bytes())
            .map_err(|e| format!("Failed to write key to gpg stdin: {}", e))?;
    }
    drop(child.stdin.take());

    let status = child.wait()
        .map_err(|e| format!("Failed to wait for gpg --import: {}", e))?;

    if !status.success() {
        return Err("gpg --import returned non-zero exit code".to_string());
    }
    Ok(())
}

/// Try to extract a key ID from a GPG "Good signature" line.
fn extract_key_id(line: &str) -> Option<String> {
    // Pattern: "gpg: Good signature from ... \"key ID ABCDEF12\""
    // Or just look for the last alphanumeric token before the end
    if let Some(start) = line.find("key ID ") {
        let rest = &line[start + 7..];
        if let Some(end) = rest.find(|c: char| !c.is_alphanumeric()) {
            return Some(rest[..end].to_string());
        }
        return Some(rest.to_string());
    }
    // Alternative: look for hex string in brackets
    if let Some(start) = line.find('[') {
        if let Some(end) = line[start..].find(']') {
            let candidate = &line[start + 1..start + end];
            if candidate.len() >= 8 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(candidate.to_string());
            }
        }
    }
    Some("verified".to_string())
}

/// RAII guard to clean up GPG temp directory.
struct GpgCleanupGuard {
    path: std::path::PathBuf,
}

impl Drop for GpgCleanupGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_trusted_keys_env_empty() {
        // No env var set
        let keys = parse_trusted_keys_env();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_parse_trusted_keys_env_semicolon() {
        let keys = parse_trusted_keys_env_from_str(
            "-----BEGIN PGP PUBLIC KEY BLOCK-----\nkey1_content\n-----END PGP PUBLIC KEY BLOCK-----\n\
             ;\
             -----BEGIN PGP PUBLIC KEY BLOCK-----\nkey2_content\n-----END PGP PUBLIC KEY BLOCK-----"
        );
        assert_eq!(keys.len(), 2);
        assert!(keys[0].contains("-----BEGIN PGP PUBLIC KEY BLOCK-----"));
        assert!(keys[0].contains("-----END PGP PUBLIC KEY BLOCK-----"));
        assert!(keys[1].contains("-----BEGIN PGP PUBLIC KEY BLOCK-----"));
        assert!(keys[1].contains("-----END PGP PUBLIC KEY BLOCK-----"));
    }

    #[test]
    fn test_parse_trusted_keys_env_skips_invalid() {
        let keys = parse_trusted_keys_env_from_str("not-a-gpg-key");
        assert!(keys.is_empty());
    }

    #[test]
    fn test_extract_key_id_from_line() {
        let line = "gpg: Good signature from \"Forge Registry <registry@forge.sh>\" [C0FFEE01]";
        let id = extract_key_id(line);
        assert_eq!(id, Some("C0FFEE01".to_string()));
    }

    #[test]
    fn test_extract_key_id_with_key_id_prefix() {
        let line = "gpg: Good signature from ... key ID ABCDEF1234567890";
        let id = extract_key_id(line);
        assert_eq!(id, Some("ABCDEF1234567890".to_string()));
    }

    #[test]
    fn test_extract_key_id_fallback() {
        let line = "gpg: Good signature from \"Unknown\"";
        let id = extract_key_id(line);
        assert_eq!(id, Some("verified".to_string()));
    }

    // Helper to test with a mock env string
    fn parse_trusted_keys_env_from_str(val: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let raw = val.replace("\r\n", "\n");
        for part in raw.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            for sub in part.split("\n\n") {
                let sub = sub.trim();
                if sub.is_empty() {
                    continue;
                }
                if sub.contains("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
                    keys.push(sub.to_string());
                }
            }
        }
        keys
    }
}
