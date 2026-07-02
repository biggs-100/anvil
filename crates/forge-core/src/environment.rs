use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;

pub trait RuntimeContextProvider: Send + Sync {
    fn workspace_root(&self) -> &Path;
    fn runtime_path(&self, name: &str) -> Option<PathBuf>;
}

use crate::resolver::{resolve_environment, resolve_environment_with_plugins, validate_environment, DoctorIssue};
use crate::secrets::{ConfigurationProvider, ResolvedEnvironment};

pub fn materialize_environment(
    ctx: &dyn RuntimeContextProvider,
    cli_overrides: &HashMap<String, String>,
    active_profile: Option<&str>,
) -> Result<ResolvedEnvironment, String> {
    let resolved = resolve_environment(ctx, cli_overrides, active_profile)?;
    materialize_environment_from_resolved(ctx, resolved)
}

/// Full version that also accepts plugin configuration providers (level 2.5).
pub fn materialize_environment_with_plugins(
    ctx: &dyn RuntimeContextProvider,
    cli_overrides: &HashMap<String, String>,
    active_profile: Option<&str>,
    plugin_config_providers: &[Box<dyn ConfigurationProvider>],
) -> Result<ResolvedEnvironment, String> {
    let resolved = resolve_environment_with_plugins(ctx, cli_overrides, active_profile, plugin_config_providers)?;
    materialize_environment_from_resolved(ctx, resolved)
}

/// Shared validation step for both materialize variants.
fn materialize_environment_from_resolved(
    ctx: &dyn RuntimeContextProvider,
    resolved: ResolvedEnvironment,
) -> Result<ResolvedEnvironment, String> {

    let toml_path = ctx.workspace_root().join("forge.toml");
    let config = if toml_path.exists() {
        crate::manifest::load_config(&toml_path).ok()
    } else {
        None
    };

    if let Some(cfg) = config {
        if let Some(config_sec) = cfg.config {
            let issues = validate_environment(&resolved.vars, &config_sec.definitions)?;
            let critical_issues: Vec<DoctorIssue> = issues
                .into_iter()
                .filter(|issue| issue.severity == "critical")
                .collect();
            if !critical_issues.is_empty() {
                let error_msgs: Vec<String> = critical_issues
                    .iter()
                    .map(|issue| format!("{} ({})", issue.message, issue.remediation))
                    .collect();
                return Err(format!("Environment validation failed:\n{}", error_msgs.join("\n")));
            }
        }
    }

    Ok(resolved)
}



pub fn find_forge_env(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let candidate = current.join("forge.env");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

pub fn parse_env_file(path: &Path) -> Result<HashMap<String, String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read env file: {}", e))?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim().to_string();
            let mut val = trimmed[pos + 1..].trim().to_string();
            // strip optional quotes
            if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                if val.len() >= 2 {
                    val = val[1..val.len() - 1].to_string();
                }
            }
            map.insert(key, val);
        }
    }
    Ok(map)
}

pub fn is_secret(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    key_lower.contains("secret") ||
    key_lower.contains("key") ||
    key_lower.contains("password") ||
    key_lower.contains("token") ||
    key_lower.contains("auth") ||
    key_lower.contains("credential") ||
    key_lower.contains("pass")
}

pub fn mask_env_vars(env_vars: &HashMap<String, String>) -> HashMap<String, String> {
    let mut masked = HashMap::new();
    for (k, v) in env_vars {
        if is_secret(k) {
            masked.insert(k.clone(), "[REDACTED]".to_string());
        } else {
            masked.insert(k.clone(), v.clone());
        }
    }
    masked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secret() {
        assert!(is_secret("API_KEY"));
        assert!(is_secret("my_secret"));
        assert!(is_secret("DB_PASSWORD"));
        assert!(!is_secret("DB_USER"));
    }

    #[test]
    fn test_mask_env_vars() {
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "supersecret123".to_string());
        env.insert("DB_USER".to_string(), "forge".to_string());
        
        let masked = mask_env_vars(&env);
        assert_eq!(masked.get("API_KEY").unwrap(), "[REDACTED]");
        assert_eq!(masked.get("DB_USER").unwrap(), "forge");
    }
}
