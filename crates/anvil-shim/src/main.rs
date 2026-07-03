use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Anvil Shim Error: Failed to resolve current executable path: {}", e);
            std::process::exit(1);
        }
    };

    let tool_name = match current_exe.file_stem().and_then(|s| s.to_str()) {
        Some(name) => name.to_lowercase(),
        None => {
            eprintln!("Anvil Shim Error: Current executable has no valid name.");
            std::process::exit(1);
        }
    };

    // Filter shim directory from PATH to prevent loops
    let shim_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));
    let raw_path = std::env::var_os("PATH").unwrap_or_default();
    let filtered_path = filter_path(&raw_path, shim_dir);

    // Look for target binary in workspace shims cache
    let mut target_binary = None;
    if let Some(cache_path) = find_shims_cache() {
        if let Some(mapped_path) = read_shims_cache(&cache_path, &tool_name) {
            if mapped_path.exists() {
                target_binary = Some(mapped_path);
            }
        }
    }

    // If not found in cache, fallback to system PATH
    let target_path = match target_binary {
        Some(path) => path,
        None => {
            let path_str = filtered_path.as_deref().unwrap_or_default();
            match find_fallback_in_path(&tool_name, path_str) {
                Some(path) => path,
                None => {
                    eprintln!("Python/Node/etc is not available. Anvil did not find a local config or a global install. Run 'anvil init' or 'anvil install <tool>'.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Execute target command
    let filtered_path_str = filtered_path.and_then(|p| p.into_string().ok());
    execute_process(&target_path, filtered_path_str.as_deref());
}

fn find_shims_cache() -> Option<PathBuf> {
    if let Ok(start_dir) = std::env::current_dir() {
        let mut current = start_dir;
        loop {
            let candidate = current.join(".anvil").join("shims.cache");
            if candidate.exists() {
                return Some(candidate);
            }
            if !current.pop() {
                break;
            }
        }
    }
    None
}

fn read_shims_cache(path: &Path, tool_name: &str) -> Option<PathBuf> {
    let content = std::fs::read_to_string(path).ok()?;
    if !content.starts_with("# anvil-shims-cache-v1") {
        return None;
    }
    parse_cache_content(&content, tool_name)
}

fn parse_cache_content(content: &str, tool_name: &str) -> Option<PathBuf> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim();
            if key.eq_ignore_ascii_case(tool_name) {
                let value = trimmed[pos + 1..].trim();
                return Some(PathBuf::from(value));
            }
        }
    }
    None
}

fn filter_path(path_var: &std::ffi::OsStr, shim_dir: &Path) -> Option<std::ffi::OsString> {
    let paths = std::env::split_paths(path_var);
    let filtered_paths: Vec<_> = paths
        .filter(|p| {
            if let (Ok(p_canon), Ok(shim_canon)) = (p.canonicalize(), shim_dir.canonicalize()) {
                p_canon != shim_canon
            } else {
                p != shim_dir
            }
        })
        .collect();
    std::env::join_paths(filtered_paths).ok()
}

fn find_fallback_in_path(tool_name: &str, filtered_path: &std::ffi::OsStr) -> Option<PathBuf> {
    let paths = std::env::split_paths(filtered_path);
    for dir in paths {
        #[cfg(windows)]
        {
            let extensions = ["exe", "cmd", "bat", "ps1"];
            for ext in &extensions {
                let candidate = dir.join(format!("{}.{}", tool_name, ext));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let candidate = dir.join(tool_name);
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if let Ok(metadata) = candidate.metadata() {
                        if metadata.mode() & 0o111 != 0 {
                            return Some(candidate);
                        }
                    }
                }
                #[cfg(not(unix))]
                return Some(candidate);
            }
        }
    }
    None
}

fn execute_process(target_path: &Path, filtered_path: Option<&str>) {
    let mut cmd = Command::new(target_path);
    cmd.args(std::env::args().skip(1));
    if let Some(path) = filtered_path {
        cmd.env("PATH", path);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("Anvil Shim Error: Failed to execute process: {}", err);
        std::process::exit(1);
    }

    #[cfg(not(unix))]
    {
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Anvil Shim Error: Failed to spawn process: {}", e);
                std::process::exit(1);
            }
        };

        let status = match child.wait() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Anvil Shim Error: Failed to wait for process: {}", e);
                std::process::exit(1);
            }
        };

        std::process::exit(status.code().unwrap_or(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_cache_content() {
        let content = r#"
            # comment line
            node = /path/to/node
            python = C:\path\to\python.exe
            BUN = /usr/bin/bun
        "#;
        assert_eq!(parse_cache_content(content, "node"), Some(PathBuf::from("/path/to/node")));
        assert_eq!(parse_cache_content(content, "python"), Some(PathBuf::from(r"C:\path\to\python.exe")));
        assert_eq!(parse_cache_content(content, "bun"), Some(PathBuf::from("/usr/bin/bun")));
        assert_eq!(parse_cache_content(content, "nonexistent"), None);
    }

    #[test]
    fn test_filter_path() {
        let temp_dir = std::env::temp_dir().join("anvil_shim_test_filter_path");
        fs::create_dir_all(&temp_dir).unwrap();
        let shim_dir = temp_dir.join("bin");
        fs::create_dir_all(&shim_dir).unwrap();

        let other_dir_1 = temp_dir.join("other1");
        fs::create_dir_all(&other_dir_1).unwrap();
        let other_dir_2 = temp_dir.join("other2");
        fs::create_dir_all(&other_dir_2).unwrap();

        // Join paths
        let paths = vec![other_dir_1.clone(), shim_dir.clone(), other_dir_2.clone()];
        let path_var = std::env::join_paths(paths).unwrap();

        let filtered = filter_path(&path_var, &shim_dir).unwrap();
        let filtered_paths = std::env::split_paths(&filtered).collect::<Vec<_>>();

        assert_eq!(filtered_paths.len(), 2);
        assert!(filtered_paths.contains(&other_dir_1));
        assert!(filtered_paths.contains(&other_dir_2));
        assert!(!filtered_paths.contains(&shim_dir));

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_find_shims_cache_traversal() {
        let temp_dir = std::env::temp_dir().join("anvil_shim_test_traversal");
        fs::create_dir_all(&temp_dir).unwrap();

        let anvil_dir = temp_dir.join(".anvil");
        fs::create_dir_all(&anvil_dir).unwrap();
        let cache_file = anvil_dir.join("shims.cache");
        fs::write(&cache_file, "# anvil-shims-cache-v1\nnode = mock_node").unwrap();

        let sub_dir = temp_dir.join("sub").join("nested");
        fs::create_dir_all(&sub_dir).unwrap();

        // Change current directory to sub_dir to test traversal
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sub_dir).unwrap();

        let found = find_shims_cache();
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), cache_file.canonicalize().unwrap());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_cache_invalidation_incorrect_header() {
        let temp_dir = std::env::temp_dir().join("anvil_shim_test_cache_invalidation");
        fs::create_dir_all(&temp_dir).unwrap();

        let cache_file = temp_dir.join("shims.cache");
        
        // 1. Without header, should return None
        fs::write(&cache_file, "node = /path/to/node\n").unwrap();
        assert_eq!(read_shims_cache(&cache_file, "node"), None);

        // 2. With incorrect header, should return None
        fs::write(&cache_file, "# anvil-shims-cache-v2\nnode = /path/to/node\n").unwrap();
        assert_eq!(read_shims_cache(&cache_file, "node"), None);

        // 3. With correct header, should parse correctly
        fs::write(&cache_file, "# anvil-shims-cache-v1\nnode = /path/to/node\n").unwrap();
        assert_eq!(read_shims_cache(&cache_file, "node"), Some(PathBuf::from("/path/to/node")));

        fs::remove_dir_all(&temp_dir).ok();
    }
}
