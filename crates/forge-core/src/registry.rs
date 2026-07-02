use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};

pub fn detect_platform() -> &'static str {
    std::env::consts::OS
}

pub fn detect_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn normalize_arch(arch: &str) -> &str {
    match arch {
        "x86_64" | "x64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => arch,
    }
}

pub fn normalize_platform(platform: &str) -> &str {
    match platform {
        "windows" | "win" | "win32" => "windows",
        "macos" | "darwin" => "macos",
        "linux" => "linux",
        _ => platform,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct HybridRegistry {
    #[serde(rename = "runtimes", default)]
    pub runtimes: Vec<RegistryEntry>,
}

impl HybridRegistry {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("Registry cache file not found: {:?}", path));
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read registry cache: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse registry cache: {}", e))
    }

    pub fn default_with_internal() -> Self {
        Self {
            runtimes: vec![
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-win-x64.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-linux-x64.tar.gz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "python".to_string(),
                    version: "3.11.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.11.0+20240107-x86_64-pc-windows-msvc-shared-install_only.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "bun".to_string(),
                    version: "1.0.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/oven-sh/bun/releases/download/bun-v1.0.0/bun-windows-x64.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "go".to_string(),
                    version: "1.21.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://go.dev/dl/go1.21.0.windows-amd64.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "rust".to_string(),
                    version: "1.75.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://static.rust-lang.org/dist/rust-1.75.0-x86_64-pc-windows-msvc.tar.gz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
            ],
        }
    }

    pub fn resolve(&self, name: &str, version_req_str: &str, platform: &str, arch: &str) -> Result<RegistryEntry, String> {
        let norm_req_str = if version_req_str.chars().all(|c| c.is_ascii_digit()) {
            format!("^{}.0.0", version_req_str)
        } else if version_req_str.starts_with('^') && version_req_str[1..].chars().all(|c| c.is_ascii_digit()) {
            format!("{}.0.0", version_req_str)
        } else if version_req_str.starts_with('~') && version_req_str[1..].chars().all(|c| c.is_ascii_digit()) {
            format!("{}.0.0", version_req_str)
        } else {
            version_req_str.to_string()
        };

        let req = semver::VersionReq::parse(&norm_req_str)
            .map_err(|e| format!("Invalid version requirement '{}': {}", version_req_str, e))?;

        let norm_plat = normalize_platform(platform);
        let norm_arch = normalize_arch(arch);

        let mut candidates: Vec<&RegistryEntry> = self.runtimes.iter()
            .filter(|entry| entry.name == name 
                && normalize_platform(&entry.platform) == norm_plat 
                && normalize_arch(&entry.arch) == norm_arch)
            .collect();

        // Fallback for Windows ARM64 to x86_64
        if candidates.is_empty() && norm_plat == "windows" && norm_arch == "aarch64" {
            candidates = self.runtimes.iter()
                .filter(|entry| entry.name == name 
                    && normalize_platform(&entry.platform) == norm_plat 
                    && normalize_arch(&entry.arch) == "x86_64")
                .collect();
        }

        let mut matching_candidates = Vec::new();
        for c in candidates {
            if let Ok(v) = semver::Version::parse(&c.version) {
                if req.matches(&v) {
                    matching_candidates.push((v, c));
                }
            }
        }

        matching_candidates.sort_by(|a, b| b.0.cmp(&a.0));

        if let Some((_, entry)) = matching_candidates.first() {
            Ok((*entry).clone())
        } else {
            Err(format!(
                "No matching version found for '{}' with requirement '{}' on platform '{}', arch '{}'",
                name, version_req_str, platform, arch
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_version_matching() {
        let registry = HybridRegistry {
            runtimes: vec![
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.9.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.9.0/node-v20.9.0-win-x64.zip".to_string(),
                    sha256: "hash20.9".to_string(),
                    size: 100,
                },
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-win-x64.zip".to_string(),
                    sha256: "hash20.10".to_string(),
                    size: 100,
                },
                RegistryEntry {
                    name: "node".to_string(),
                    version: "18.15.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v18.15.0/node-v18.15.0-win-x64.zip".to_string(),
                    sha256: "hash18.15".to_string(),
                    size: 100,
                },
            ],
        };

        let matched = registry.resolve("node", "^20", "windows", "x86_64").unwrap();
        assert_eq!(matched.version, "20.10.0");

        let matched_tilde = registry.resolve("node", "~20.9", "windows", "x86_64").unwrap();
        assert_eq!(matched_tilde.version, "20.9.0");

        let err_match = registry.resolve("node", "^21", "windows", "x86_64");
        assert!(err_match.is_err());
    }
}
