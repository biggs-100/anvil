use std::fs;
use std::path::Path;
use crate::types::Lockfile;

pub fn load_lockfile(path: &Path) -> Result<Lockfile, String> {
    if !path.exists() {
        return Ok(Lockfile::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read lockfile: {}", e))?;
    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse lockfile: {}", e))
}

pub fn save_lockfile(path: &Path, lockfile: &Lockfile) -> Result<(), String> {
    let mut sorted_lockfile = lockfile.clone();
    sorted_lockfile.runtimes.sort_by(|a, b| a.name.cmp(&b.name));
    
    let content = toml::to_string_pretty(&sorted_lockfile)
        .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;
    fs::write(path, content)
        .map_err(|e| format!("Failed to write lockfile: {}", e))
}

