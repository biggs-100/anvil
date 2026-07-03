use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::manifest::PackagesConfig;

/// Install pip packages from the configured requirements file.
///
/// Returns `Ok(())` if no `[packages]` section exists, or if pip install
/// succeeds. Returns `Err(String)` when pip is configured but the python
/// runtime is missing, the requirements file is missing, or pip exits
/// with a non-zero status.
pub fn install_pip_deps(
    config: &PackagesConfig,
    workspace_root: &Path,
    cache_dir: &Path,
) -> Result<(), String> {
    let requirements_path = match &config.pip {
        Some(path) => path,
        None => return Ok(()), // no pip section = no-op
    };

    // Resolve the anvil-managed python binary from cache
    let python_binary = resolve_python_binary(cache_dir)?;

    // Resolve the requirements file path relative to workspace_root
    let requirements_file = workspace_root.join(requirements_path);
    if !requirements_file.exists() {
        return Err(format!(
            "Requirements file not found: {}",
            requirements_file.display()
        ));
    }

    // Build the bin_dirs for the python runtime's extracted directory
    let python_bin_dir = python_binary
        .parent()
        .ok_or_else(|| "Could not determine python bin directory".to_string())?;

    let bin_dirs = vec![python_bin_dir.to_path_buf()];

    // Build the pip install command
    let python_exe = python_binary
        .to_string_lossy()
        .to_string();
    let pip_args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-r".to_string(),
        requirements_file.to_string_lossy().to_string(),
    ];

    println!("Installing pip packages from '{}'...", requirements_file.display());

    let exit_code = crate::launcher::run_command_in_env(
        &python_exe,
        &pip_args,
        &HashMap::new(), // no extra env vars
        &bin_dirs,
    )?;

    if exit_code != 0 {
        return Err(format!(
            "pip install exited with code {}",
            exit_code
        ));
    }

    Ok(())
}

/// Find the anvil-managed python binary in the cache directory.
///
/// Looks for `{cache_dir}/python/*/extracted/bin/python3` (or `python.exe`
/// on Windows). Returns an error if no python runtime is found.
fn resolve_python_binary(cache_dir: &Path) -> Result<PathBuf, String> {
    let python_dir = cache_dir.join("python");
    if !python_dir.exists() {
        return Err(
            "No python runtime found in anvil cache. Run 'anvil up' to sync runtimes first."
                .to_string(),
        );
    }

    let mut entries: Vec<_> = std::fs::read_dir(&python_dir)
        .map_err(|e| format!("Failed to read python cache directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect();

    // Sort by entry name (version) descending to pick the latest
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for entry in entries {
        let extracted_bin = entry.path().join("extracted").join("bin");
        if !extracted_bin.exists() {
            continue;
        }

        let binary_name = if cfg!(windows) {
            "python.exe"
        } else {
            "python3"
        };

        let candidate = extracted_bin.join(binary_name);
        if candidate.exists() {
            return Ok(candidate);
        }

        // Fallback: try just "python" on non-Windows or "python.exe" alternative
        let fallback = if cfg!(windows) {
            extracted_bin.join("python")
        } else {
            extracted_bin.join("python")
        };
        if fallback.exists() {
            return Ok(fallback);
        }
    }

    Err(
        "No python runtime found in anvil cache. Run 'anvil up' to sync runtimes first."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PackagesConfig;

    #[test]
    fn test_install_pip_deps_no_packages_config() {
        // No pip field means no-op — should return Ok(())
        let config = PackagesConfig { pip: None };
        let temp_dir = std::env::temp_dir().join("anvil_pkgs_test_noop");
        let workspace = temp_dir.join("workspace");
        let cache = temp_dir.join("cache");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&cache).unwrap();

        let result = install_pip_deps(&config, &workspace, &cache);
        assert!(result.is_ok(), "Expected Ok for no pip config");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_install_pip_deps_missing_python() {
        // Pip configured but no python in cache — should return error
        let config = PackagesConfig {
            pip: Some("requirements.txt".to_string()),
        };
        let temp_dir = std::env::temp_dir().join("anvil_pkgs_test_no_python");
        let workspace = temp_dir.join("workspace");
        let cache = temp_dir.join("cache");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&cache).unwrap();

        // Create a requirements file so we pass that check
        std::fs::write(workspace.join("requirements.txt"), "requests\n").unwrap();

        let result = install_pip_deps(&config, &workspace, &cache);
        assert!(result.is_err(), "Expected error for missing python runtime");
        let err = result.unwrap_err();
        assert!(
            err.contains("No python runtime found"),
            "Error should mention missing python: {}",
            err
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_install_pip_deps_missing_requirements() {
        // Pip configured but requirements file missing — should return error
        let config = PackagesConfig {
            pip: Some("requirements.txt".to_string()),
        };
        let temp_dir = std::env::temp_dir().join("anvil_pkgs_test_missing_reqs");
        let workspace = temp_dir.join("workspace");
        let cache = temp_dir.join("cache");

        // Create a mock python runtime so we pass the python check
        let python_bin = cache.join("python").join("3.12.0").join("extracted").join("bin");
        std::fs::create_dir_all(&python_bin).unwrap();
        #[cfg(windows)]
        let binary_name = "python.exe";
        #[cfg(not(windows))]
        let binary_name = "python3";
        std::fs::write(python_bin.join(binary_name), "mock python").unwrap();

        // Do NOT create requirements.txt — that's what we're testing

        let result = install_pip_deps(&config, &workspace, &cache);
        assert!(result.is_err(), "Expected error for missing requirements file");
        let err = result.unwrap_err();
        assert!(
            err.contains("Requirements file not found"),
            "Error should mention missing file: {}",
            err
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_packages_config_toml_deserialization() {
        let toml_str = r#"
[packages]
pip = "reqs.txt"
"#;
        // Parse as value first, then extract the packages table
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let config: PackagesConfig = value
            .get("packages")
            .unwrap()
            .clone()
            .try_into()
            .unwrap();
        assert_eq!(config.pip, Some("reqs.txt".to_string()));
    }

    #[test]
    fn test_packages_config_embedded_in_anvil_config() {
        let toml_str = r#"
[runtimes]
python = "3.12.0"

[packages]
pip = "requirements.txt"
"#;
        let config: crate::manifest::AnvilConfig = toml::from_str(toml_str).unwrap();
        assert!(config.packages.is_some());
        assert_eq!(
            config.packages.as_ref().unwrap().pip,
            Some("requirements.txt".to_string())
        );
    }

    #[test]
    fn test_anvil_config_no_packages() {
        // No [packages] section => packages should be None
        let toml_str = r#"
[runtimes]
node = "20.11.0"
"#;
        let config: crate::manifest::AnvilConfig = toml::from_str(toml_str).unwrap();
        assert!(config.packages.is_none());
    }

    #[test]
    fn test_resolve_python_binary_empty_cache() {
        let temp_dir = std::env::temp_dir().join("anvil_pkgs_resolve_empty");
        let cache = temp_dir.join("cache");
        std::fs::create_dir_all(&cache).unwrap();

        let result = resolve_python_binary(&cache);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No python runtime found"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_resolve_python_binary_finds_python() {
        let temp_dir = std::env::temp_dir().join("anvil_pkgs_resolve_find");
        let cache = temp_dir.join("cache");

        // Create a mock python runtime structure
        let python_dir = cache.join("python").join("3.12.0").join("extracted").join("bin");
        std::fs::create_dir_all(&python_dir).unwrap();

        #[cfg(windows)]
        let binary_name = "python.exe";
        #[cfg(not(windows))]
        let binary_name = "python3";

        std::fs::write(python_dir.join(binary_name), "mock python binary").unwrap();

        let result = resolve_python_binary(&cache);
        assert!(result.is_ok(), "Should find python binary: {:?}", result);
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains(binary_name));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
