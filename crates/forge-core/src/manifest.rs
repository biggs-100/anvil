// TODO: tests — requires TOML parsing fixtures
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use serde::{Serialize, Deserialize};
use crate::policy::PolicyConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigDefinition {
    #[serde(rename = "type")]
    pub val_type: Option<String>,
    #[serde(default)]
    pub required: bool,
    pub default: Option<toml::Value>,
    pub pattern: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigSection {
    #[serde(default)]
    pub definitions: HashMap<String, ConfigDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileSection {
    #[serde(default)]
    pub env: HashMap<String, toml::Value>,
}

/// A runtime entry from `forge.toml` — either a bare version string
/// or a pinned object with optional `sha256`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RuntimeEntry {
    /// Bare version string: `node = "20.11.0"`
    Bare(String),
    /// Pinned object: `node = { version = "20.11.0", sha256 = "abc..." }`
    Pinned {
        version: String,
        #[serde(default)]
        sha256: Option<String>,
    },
}

impl RuntimeEntry {
    /// Get the version string from either variant.
    pub fn version(&self) -> &str {
        match self {
            RuntimeEntry::Bare(v) => v,
            RuntimeEntry::Pinned { version, .. } => version,
        }
    }

    /// Get the optional sha256 pin from the Pinned variant.
    pub fn sha256(&self) -> Option<&str> {
        match self {
            RuntimeEntry::Bare(_) => None,
            RuntimeEntry::Pinned { sha256, .. } => sha256.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackagesConfig {
    #[serde(default)]
    pub pip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    #[serde(default)]
    pub runtimes: HashMap<String, RuntimeEntry>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub config: Option<ConfigSection>,
    #[serde(default)]
    pub profile: Option<HashMap<String, ProfileSection>>,
    #[serde(default)]
    pub policy: Option<PolicyConfig>,
    #[serde(default)]
    pub packages: Option<PackagesConfig>,
}


pub fn find_forge_toml(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let candidate = current.join("forge.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

pub fn load_config(toml_path: &Path) -> Result<ForgeConfig, String> {
    let content = fs::read_to_string(toml_path)
        .map_err(|e| format!("Failed to read forge.toml: {}", e))?;
    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse forge.toml: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_entry_bare_string() {
        let toml_str = r#"
[runtimes]
node = "20.11.0"
python = "3.12.0"
"#;
        let config: ForgeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.runtimes.len(), 2);

        let node = config.runtimes.get("node").unwrap();
        assert_eq!(node.version(), "20.11.0");
        assert_eq!(node.sha256(), None);

        let python = config.runtimes.get("python").unwrap();
        assert_eq!(python.version(), "3.12.0");
    }

    #[test]
    fn test_runtime_entry_pinned_object() {
        let toml_str = r#"
[runtimes]
node = { version = "20.11.0", sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" }
python = { version = "3.12.0" }
"#;
        let config: ForgeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.runtimes.len(), 2);

        let node = config.runtimes.get("node").unwrap();
        assert_eq!(node.version(), "20.11.0");
        assert_eq!(node.sha256(), Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));

        let python = config.runtimes.get("python").unwrap();
        assert_eq!(python.version(), "3.12.0");
        assert_eq!(python.sha256(), None);
    }

    #[test]
    fn test_runtime_entry_mixed_syntax() {
        let toml_str = r#"
[runtimes]
node = { version = "20.11.0", sha256 = "abc123" }
python = "3.12.0"
go = { version = "1.21.0", sha256 = "def456" }
"#;
        let config: ForgeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.runtimes.len(), 3);

        // Bare string
        let python = config.runtimes.get("python").unwrap();
        assert!(matches!(python, RuntimeEntry::Bare(_)));
        assert_eq!(python.version(), "3.12.0");

        // Pinned with hash
        let node = config.runtimes.get("node").unwrap();
        assert_eq!(node.version(), "20.11.0");
        assert_eq!(node.sha256(), Some("abc123"));

        let go = config.runtimes.get("go").unwrap();
        assert_eq!(go.version(), "1.21.0");
        assert_eq!(go.sha256(), Some("def456"));
    }

    #[test]
    fn test_serde_roundtrip() {
        // Test roundtrip via ForgeConfig (TOML requires a table root)
        let config: ForgeConfig = toml::from_str(r#"
            [runtimes]
            node = "20.11.0"
        "#).unwrap();
        let node = config.runtimes.get("node").unwrap();
        assert_eq!(node, &RuntimeEntry::Bare("20.11.0".to_string()));

        // Re-serialize as TOML and parse back
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: ForgeConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.runtimes.get("node"),
            Some(&RuntimeEntry::Bare("20.11.0".to_string()))
        );

        // Pinned entry roundtrip
        let config2: ForgeConfig = toml::from_str(r#"
            [runtimes]
            node = { version = "20.11.0", sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" }
        "#).unwrap();
        let node2 = config2.runtimes.get("node").unwrap();
        assert!(matches!(node2, RuntimeEntry::Pinned { .. }));
        assert_eq!(node2.sha256(), Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));

        // Re-serialize and parse back
        let serialized2 = toml::to_string(&config2).unwrap();
        let deserialized2: ForgeConfig = toml::from_str(&serialized2).unwrap();
        assert_eq!(
            deserialized2.runtimes.get("node").and_then(|r| r.sha256()),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }
}
