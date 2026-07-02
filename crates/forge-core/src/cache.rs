use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use sha2::{Sha256, Digest};
use crate::types::Lockfile;

pub fn get_cache_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    Ok(home.join(".forge").join("runtimes"))
}

pub fn find_bin_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut bin_dirs = Vec::new();
    let mut check_dirs = vec![dir.to_path_buf()];
    
    while let Some(curr) = check_dirs.pop() {
        let mut has_executable = false;
        if let Ok(entries) = fs::read_dir(&curr) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name_lower == "node" || name_lower == "node.exe"
                            || name_lower == "python" || name_lower == "python.exe" || name_lower == "python3" || name_lower == "python3.exe"
                            || name_lower == "bun" || name_lower == "bun.exe"
                            || name_lower == "go" || name_lower == "go.exe"
                            || name_lower == "cargo" || name_lower == "cargo.exe"
                            || name_lower == "rustc" || name_lower == "rustc.exe"
                        {
                            has_executable = true;
                        }
                    }
                } else if path.is_dir() {
                    check_dirs.push(path);
                }
            }
        }
        if has_executable {
            bin_dirs.push(curr);
        }
    }
    bin_dirs
}

pub fn generate_shims_cache_map(lockfile: &Lockfile, cache_dir: &Path) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for runtime in &lockfile.runtimes {
        let extract_dir = cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
        let bin_dirs = find_bin_dirs(&extract_dir);
        for bin_dir in bin_dirs {
            if let Ok(entries) = fs::read_dir(&bin_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            let name_lower = filename.to_lowercase();
                            let stem_lower = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                            
                            let targets = [
                                "node", "python", "python3", "bun", "go", "cargo", "rustc", "rust"
                            ];
                            let is_target = targets.iter().any(|&t| name_lower == t || stem_lower == t);
                            if is_target {
                                map.insert(stem_lower.clone(), path.clone());
                                map.insert(name_lower.clone(), path.clone());
                                
                                if stem_lower == "python" || stem_lower == "python3" {
                                    map.insert("python".to_string(), path.clone());
                                    map.insert("python3".to_string(), path.clone());
                                }
                                if stem_lower == "rustc" {
                                    map.insert("rust".to_string(), path.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

pub fn write_shims_cache_file(cache_file_path: &Path, map: &HashMap<String, PathBuf>) -> Result<(), String> {
    if let Some(parent) = cache_file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .forge directory: {}", e))?;
    }
    
    let mut entries: Vec<(&String, &PathBuf)> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    
    let mut hasher = Sha256::new();
    for (k, v) in &entries {
        hasher.update(k.as_bytes());
        hasher.update(v.to_string_lossy().as_bytes());
    }
    let signature = hex::encode(hasher.finalize());
    
    let mut content = String::new();
    content.push_str("# forge-shims-cache-v1\n");
    content.push_str("# generated_at: 2026-07-01T07:45:21-05:00\n");
    content.push_str(&format!("# version_signature: {}\n", &signature[..8]));
    
    for (k, v) in entries {
        content.push_str(&format!("{} = {}\n", k, v.display()));
    }
    
    fs::write(cache_file_path, content)
        .map_err(|e| format!("Failed to write shims cache: {}", e))?;
        
    Ok(())
}

pub fn regenerate_shims_cache(lockfile: &Lockfile, cache_dir: &Path, workspace_dir: &Path) -> Result<(), String> {
    let cache_file_path = workspace_dir.join(".forge").join("shims.cache");
    let map = generate_shims_cache_map(lockfile, cache_dir);
    write_shims_cache_file(&cache_file_path, &map)
}

pub fn append_to_gitignore(workspace_dir: &Path) -> Result<(), String> {
    let gitignore_path = workspace_dir.join(".gitignore");
    let mut content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .map_err(|e| format!("Failed to read .gitignore: {}", e))?
    } else {
        String::new()
    };

    let mut modified = false;
    let entries = [".forge/shims.cache", ".forge/state.json"];
    for entry in &entries {
        if !content.lines().any(|line| line.trim() == *entry) {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(entry);
            content.push('\n');
            modified = true;
        }
    }

    if modified {
        fs::write(&gitignore_path, content)
            .map_err(|e| format!("Failed to write .gitignore: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_to_gitignore() {
        let temp_dir = std::env::temp_dir().join("forge_gitignore_test");
        fs::create_dir_all(&temp_dir).unwrap();
        
        let gitignore_path = temp_dir.join(".gitignore");
        fs::write(&gitignore_path, "target/\n.DS_Store").unwrap();
        
        append_to_gitignore(&temp_dir).unwrap();
        
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".forge/shims.cache"));
        assert!(content.contains(".forge/state.json"));
        
        let prev_len = content.len();
        append_to_gitignore(&temp_dir).unwrap();
        let content_after = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content_after.len(), prev_len);
        
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_shims_cache_serialization() {
        let temp_dir = std::env::temp_dir().join("forge_core_shims_cache_test");
        fs::create_dir_all(&temp_dir).unwrap();
        
        let cache_file = temp_dir.join("shims.cache");
        let mut map = HashMap::new();
        map.insert("node".to_string(), PathBuf::from("/usr/bin/node"));
        map.insert("python".to_string(), PathBuf::from("/usr/bin/python"));
        
        write_shims_cache_file(&cache_file, &map).unwrap();
        
        let content = fs::read_to_string(&cache_file).unwrap();
        assert!(content.contains("node = /usr/bin/node"));
        assert!(content.contains("python = /usr/bin/python"));
        assert!(content.contains("version_signature"));
        
        fs::remove_dir_all(&temp_dir).ok();
    }
}
