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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    #[serde(default)]
    pub runtimes: HashMap<String, String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub config: Option<ConfigSection>,
    #[serde(default)]
    pub profile: Option<HashMap<String, ProfileSection>>,
    #[serde(default)]
    pub policy: Option<PolicyConfig>,
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
