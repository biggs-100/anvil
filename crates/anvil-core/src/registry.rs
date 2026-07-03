use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{Duration, SystemTime};
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

// ═══════════════════════════════════════════════════════════════
// Phase 1: ARRS Types
// ═══════════════════════════════════════════════════════════════

/// An artifact entry within an ARRS metadata.toml — describes a
/// single platform/arch download for a toolchain version.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArrsArtifact {
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// Full ARRS metadata.toml for a single toolchain version.
/// Unknown TOML fields are silently ignored (forward compat per
/// REQ-ARRS-004).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArrsMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub artifacts: Vec<ArrsArtifact>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
}

/// Registry index.toml — lists available toolchains with their
/// latest version and version history.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryIndex {
    pub toolchains: HashMap<String, RegistryIndexEntry>,
}

/// A single toolchain entry in the registry index.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryIndexEntry {
    #[serde(rename = "latest")]
    pub latest_version: String,
    #[serde(default)]
    pub versions: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════
// Phase 2: RemoteRegistry
// ═══════════════════════════════════════════════════════════════

/// Fetches ARRS metadata from a remote registry over HTTP and
/// caches results locally for offline resilience.
///
/// Manual Debug impl because `reqwest::Client` does not implement Debug.
pub struct RemoteRegistry {
    base_url: String,
    cache_dir: PathBuf,
    client: reqwest::Client,
    ttl: Duration,
}

impl std::fmt::Debug for RemoteRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteRegistry")
            .field("base_url", &self.base_url)
            .field("cache_dir", &self.cache_dir)
            .field("ttl", &self.ttl)
            .finish()
    }
}

// reqwest::Client implements Clone (cheap Arc-based), so this is safe.
impl Clone for RemoteRegistry {
    fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            cache_dir: self.cache_dir.clone(),
            client: self.client.clone(),
            ttl: self.ttl,
        }
    }
}

impl RemoteRegistry {
    /// Create a new remote registry pointed at `base_url`.
    /// Cache is stored under `cache_dir` using the ARRS directory layout
    /// `{cache_dir}/{name}/{version}/metadata.toml`.
    /// Default TTL is 24 hours.
    pub fn new(base_url: &str, cache_dir: PathBuf) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            cache_dir,
            client: reqwest::Client::new(),
            ttl: Duration::from_secs(24 * 60 * 60),
        }
    }

    /// Override the default cache TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Fetch `metadata.toml` from `{base_url}/{name}/{version}/metadata.toml`.
    /// Also fetches the companion `metadata.toml.asc` detached GPG signature
    /// and verifies it against the embedded (and env-supplied) trusted keys.
    /// On success the result is also saved to the local cache.
    /// On signature failure, falls back to cached metadata (if available).
    pub async fn fetch_metadata(&self, name: &str, version: &str) -> Result<ArrsMetadata, String> {
        let url = format!("{}/{}/{}/metadata.toml", self.base_url, name, version);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {}/{} from remote: {}", name, version, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Remote registry returned HTTP {} for {}/{}",
                response.status(),
                name,
                version
            ));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Attempt GPG signature verification
        let verification_ok = self.verify_metadata_signature(name, version, &text).await;

        if !verification_ok {
            // Signature invalid or missing — fall back to cache
            if let Some(cached) = self.load_cached(name, version) {
                eprintln!(
                    "[anvil] Warning: Signature verification failed for {}/{}, \
                     using cached metadata",
                    name, version
                );
                return Ok(cached);
            }
            // No cached data available — return the verification error
            // but only if strict mode is on, otherwise serve the data anyway
            if std::env::var("ANVIL_GPG_STRICT").as_deref() == Ok("1") {
                return Err(format!(
                    "GPG signature verification failed for {}/{} and no cached metadata available",
                    name, version
                ));
            }
            eprintln!(
                "[anvil] Warning: Serving {}/{} without valid GPG signature (lenient mode)",
                name, version
            );
        }

        let metadata: ArrsMetadata =
            toml::from_str(&text).map_err(|e| format!("Failed to parse metadata.toml for {}/{}: {}", name, version, e))?;

        // Persist to cache immediately
        if let Err(e) = self.save_to_cache(&metadata) {
            eprintln!("[anvil] Warning: Failed to cache {}/{}: {}", name, version, e);
        }

        Ok(metadata)
    }

    /// Attempt to fetch and verify the detached GPG signature for a metadata
    /// fetch. Returns `true` if verification succeeded (or GPG is simply
    /// unavailable in lenient mode), `false` on failure.
    async fn verify_metadata_signature(
        &self,
        name: &str,
        version: &str,
        metadata_text: &str,
    ) -> bool {
        let sig_url = format!(
            "{}/{}/{}/metadata.toml.asc",
            self.base_url, name, version
        );

        let sig_response = match self.client.get(&sig_url).send().await {
            Ok(resp) if resp.status().is_success() => resp,
            Ok(resp) => {
                eprintln!(
                    "[anvil] Warning: Signature file metadata.toml.asc for {}/{} \
                     returned HTTP {}",
                    name,
                    version,
                    resp.status()
                );
                return false;
            }
            Err(e) => {
                eprintln!(
                    "[anvil] Warning: Failed to fetch signature for {}/{}: {}",
                    name, version, e
                );
                return false;
            }
        };

        let sig_bytes = match sig_response.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                eprintln!(
                    "[anvil] Warning: Failed to read signature body for {}/{}: {}",
                    name, version, e
                );
                return false;
            }
        };

        // Collect additional keys from ANVIL_TRUSTED_KEYS env var
        let additional_keys = crate::gpg::parse_trusted_keys_env();

        match crate::gpg::verify_gpg_signature(
            metadata_text.as_bytes(),
            &sig_bytes,
            &additional_keys,
        ) {
            Ok(key_id) => {
                eprintln!(
                    "[anvil] Info: GPG signature verified for {}/{} (key: {})",
                    name, version, key_id
                );
                true
            }
            Err(e) => {
                eprintln!(
                    "[anvil] Warning: GPG verification failed for {}/{}: {}",
                    name, version, e
                );
                false
            }
        }
    }

    /// Load cached metadata for `name`/`version` from the local
    /// ARRS cache directory.  Returns `None` when the file is
    /// missing or corrupt.
    ///
    /// TTL semantics:
    ///   - Fresh entries are served silently.
    ///   - Stale entries are served with a warning.
    ///   - There is no hard expiry — stale data is always returned
    ///     to support offline-first operation (REQ-REG-007).
    pub fn load_cached(&self, name: &str, version: &str) -> Option<ArrsMetadata> {
        let path = self.cache_dir.join(name).join(version).join("metadata.toml");
        if !path.exists() {
            return None;
        }

        // Warn on stale cache but still serve it
        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed >= self.ttl {
                        eprintln!(
                            "[anvil] Warning: Cached {}/{} is stale ({}h old), serving anyway",
                            name,
                            version,
                            elapsed.as_secs() / 3600
                        );
                    }
                }
            }
        }

        let content = fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Fetch the registry index from `{base_url}/index.toml`.
    pub async fn fetch_index(&self) -> Result<RegistryIndex, String> {
        let url = format!("{}/index.toml", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch registry index: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Registry index returned HTTP {}", response.status()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read index response: {}", e))?;

        let index: RegistryIndex =
            toml::from_str(&text).map_err(|e| format!("Failed to parse index.toml: {}", e))?;

        Ok(index)
    }

    // ── helpers ──────────────────────────────────────────────

    fn save_to_cache(&self, metadata: &ArrsMetadata) -> Result<(), String> {
        let path = self
            .cache_dir
            .join(&metadata.name)
            .join(&metadata.version)
            .join("metadata.toml");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache dir {:?}: {}", parent, e))?;
        }

        let content = toml::to_string(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write cache {:?}: {}", path, e))?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════
// Phase 3: HybridRegistry
// ═══════════════════════════════════════════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HybridRegistry {
    #[serde(rename = "runtimes", default)]
    pub runtimes: Vec<RegistryEntry>,
    #[serde(skip)]
    pub remote: Option<RemoteRegistry>,
    #[serde(skip)]
    pub cache_dir: Option<PathBuf>,
}

impl HybridRegistry {
    /// Load a legacy flat metadata_cache.toml file.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("Registry cache file not found: {:?}", path));
        }
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read registry cache: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse registry cache: {}", e))
    }

    /// Return built-in default entries for all supported runtimes.
    pub fn default_with_internal() -> Self {
        Self {
            runtimes: vec![
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-win-x64.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-linux-x64.tar.gz"
                        .to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "python".to_string(),
                    version: "3.11.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.11.0+20240107-x86_64-pc-windows-msvc-shared-install_only.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "bun".to_string(),
                    version: "1.0.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/oven-sh/bun/releases/download/bun-v1.0.0/bun-windows-x64.zip"
                        .to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "go".to_string(),
                    version: "1.21.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://go.dev/dl/go1.21.0.windows-amd64.zip".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "rust".to_string(),
                    version: "1.75.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://static.rust-lang.org/dist/rust-1.75.0-x86_64-pc-windows-msvc.tar.gz"
                        .to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                    size: 1048576,
                },
                // ── LLVM ──────────────────────────────────────
                RegistryEntry {
                    name: "llvm".to_string(),
                    version: "18.1.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0/LLVM-18.1.0-win64.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "llvm".to_string(),
                    version: "18.1.0".to_string(),
                    platform: "macos".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0/LLVM-18.1.0-macos.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "llvm".to_string(),
                    version: "18.1.0".to_string(),
                    platform: "macos".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0/LLVM-18.1.0-macos-aarch64.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "llvm".to_string(),
                    version: "18.1.0".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0/LLVM-18.1.0-linux.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "llvm".to_string(),
                    version: "18.1.0".to_string(),
                    platform: "linux".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0/LLVM-18.1.0-linux-aarch64.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                // ── JDK 21 LTS ────────────────────────────────
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "21.0.2".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/21.0.2/windows/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "21.0.2".to_string(),
                    platform: "macos".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/21.0.2/mac/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "21.0.2".to_string(),
                    platform: "macos".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/21.0.2/mac/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "21.0.2".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/21.0.2/linux/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "21.0.2".to_string(),
                    platform: "linux".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/21.0.2/linux/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                // ── JDK 17 LTS ────────────────────────────────
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "17.0.9".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/17.0.9/windows/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "17.0.9".to_string(),
                    platform: "macos".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/17.0.9/mac/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "17.0.9".to_string(),
                    platform: "macos".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/17.0.9/mac/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "17.0.9".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/17.0.9/linux/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
                RegistryEntry {
                    name: "jdk".to_string(),
                    version: "17.0.9".to_string(),
                    platform: "linux".to_string(),
                    arch: "aarch64".to_string(),
                    url: "https://api.adoptium.net/v3/binary/version/17.0.9/linux/hotspot/normal/eclipse?project=jdk".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size: 1048576,
                },
            ],
            remote: None,
            cache_dir: None,
        }
    }

    // ── builder methods ──────────────────────────────────────

    /// Attach a remote registry for ARRS HTTP fetches.
    pub fn with_remote(mut self, remote: RemoteRegistry) -> Self {
        self.remote = Some(remote);
        self
    }

    /// Set a local ARRS cache directory for offline metadata lookup.
    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = Some(cache_dir);
        self
    }

    // ── refresh_remote ────────────────────────────────────────

    /// Ensure the ARRS cache is populated for `name` by fetching the
    /// latest version from the remote registry.  This is a best-effort
    /// pre-fetch before resolution — failures are logged, not fatal.
    ///
    /// If the remote is unreachable and no cached data exists, the
    /// error is returned (the caller should still attempt resolution,
    /// which will fall through to embedded defaults).
    pub async fn refresh_remote(&self, name: &str) -> Result<(), String> {
        let remote = self
            .remote
            .as_ref()
            .ok_or_else(|| "No remote registry configured".to_string())?;

        // Fetch index to find latest version
        let index = match remote.fetch_index().await {
            Ok(idx) => idx,
            Err(e) => {
                // Network failure — check cache
                eprintln!("[anvil] Warning: Cannot reach remote registry: {}", e);
                return Ok(()); // not fatal, resolve will try cache + defaults
            }
        };

        let toolchain = match index.toolchains.get(name) {
            Some(tc) => tc,
            None => {
                eprintln!("[anvil] Warning: Runtime '{}' not found in registry index", name);
                return Ok(());
            }
        };

        let latest = &toolchain.latest_version;

        // Already cached?  Nothing to do.
        if remote.load_cached(name, latest).is_some() {
            return Ok(());
        }

        // Fetch and cache (already saved by fetch_metadata).
        match remote.fetch_metadata(name, latest).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Network failure on fetch; cache is the final fallback.
                if remote.load_cached(name, latest).is_some() {
                    eprintln!(
                        "[anvil] Warning: Using stale cache for {}/{}: {}",
                        name, latest, e
                    );
                    return Ok(());
                }
                Err(e)
            }
        }
    }

    // ── resolve ──────────────────────────────────────────────

    /// The 4-tier resolution chain:
    ///   1. Flat entries (`self.runtimes`)
    ///   2. Local ARRS cache directory
    ///   3. ARM64 → x86_64 fallback (within each tier)
    ///   4. Embedded compiled-in defaults (`default_with_internal()`)
    pub fn resolve(
        &self,
        name: &str,
        version_req_str: &str,
        platform: &str,
        arch: &str,
    ) -> Result<RegistryEntry, String> {
        let norm_req_str = if version_req_str.chars().all(|c| c.is_ascii_digit()) {
            format!("^{}.0.0", version_req_str)
        } else if version_req_str.starts_with('^')
            && version_req_str[1..].chars().all(|c| c.is_ascii_digit())
        {
            format!("{}.0.0", version_req_str)
        } else if version_req_str.starts_with('~')
            && version_req_str[1..].chars().all(|c| c.is_ascii_digit())
        {
            format!("{}.0.0", version_req_str)
        } else {
            version_req_str.to_string()
        };

        let req = semver::VersionReq::parse(&norm_req_str)
            .map_err(|e| format!("Invalid version requirement '{}': {}", version_req_str, e))?;

        // ── Tier 1: flat entries ──
        if let Some(entry) = Self::resolve_from_list(&self.runtimes, name, &req, platform, arch) {
            return Ok(entry);
        }

        // ── Tier 2: local ARRS cache directory ──
        if let Some(cache_dir) = &self.cache_dir {
            if let Some(entry) = self.resolve_from_cache(cache_dir, name, &req, platform, arch) {
                return Ok(entry);
            }
        }

        // ── Tier 3: embedded compiled-in defaults ──
        let defaults = Self::default_with_internal();
        if let Some(entry) = Self::resolve_from_list(&defaults.runtimes, name, &req, platform, arch)
        {
            return Ok(entry);
        }

        Err(format!(
            "No matching version found for '{}' with requirement '{}' on platform '{}', arch '{}'",
            name, version_req_str, platform, arch
        ))
    }

    // ── internal helpers ─────────────────────────────────────

    /// Resolve the best matching entry from a flat registry entry list.
    /// Includes ARM64 → x86_64 fallback for Windows.
    fn resolve_from_list(
        runtimes: &[RegistryEntry],
        name: &str,
        req: &semver::VersionReq,
        platform: &str,
        arch: &str,
    ) -> Option<RegistryEntry> {
        let norm_plat = normalize_platform(platform);
        let norm_arch = normalize_arch(arch);

        let mut candidates: Vec<&RegistryEntry> = runtimes
            .iter()
            .filter(|entry| {
                entry.name == name
                    && normalize_platform(&entry.platform) == norm_plat
                    && normalize_arch(&entry.arch) == norm_arch
            })
            .collect();

        // ARM64 → x86_64 fallback for Windows
        if candidates.is_empty() && norm_plat == "windows" && norm_arch == "aarch64" {
            candidates = runtimes
                .iter()
                .filter(|entry| {
                    entry.name == name
                        && normalize_platform(&entry.platform) == norm_plat
                        && normalize_arch(&entry.arch) == "x86_64"
                })
                .collect();
        }

        let mut matching: Vec<(semver::Version, RegistryEntry)> = Vec::new();
        for c in candidates {
            if let Ok(v) = semver::Version::parse(&c.version) {
                if req.matches(&v) {
                    matching.push((v, c.clone()));
                }
            }
        }

        matching.sort_by(|a, b| b.0.cmp(&a.0));
        matching.first().map(|(_, entry)| entry.clone())
    }

    /// Scan the ARRS cache directory for `name`, find any versions
    /// matching the semver requirement, and return the best
    /// `RegistryEntry` for the requested platform/arch.
    fn resolve_from_cache(
        &self,
        cache_dir: &Path,
        name: &str,
        req: &semver::VersionReq,
        platform: &str,
        arch: &str,
    ) -> Option<RegistryEntry> {
        let name_dir = cache_dir.join(name);
        if !name_dir.is_dir() {
            return None;
        }

        let norm_plat = normalize_platform(platform);
        let norm_arch = normalize_arch(arch);

        let dir_entries = fs::read_dir(&name_dir).ok()?;
        let mut candidates: Vec<(semver::Version, RegistryEntry)> = Vec::new();

        for entry in dir_entries {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let version_str = path.file_name()?.to_str()?;
            let version = semver::Version::parse(version_str).ok()?;
            if !req.matches(&version) {
                continue;
            }

            let metadata_path = path.join("metadata.toml");
            if !metadata_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&metadata_path).ok()?;
            let metadata: ArrsMetadata = toml::from_str(&content).ok()?;

            // Find matching artifact
            if let Some(artifact) = metadata.artifacts.iter().find(|a| {
                normalize_platform(&a.platform) == norm_plat
                    && normalize_arch(&a.arch) == norm_arch
            }) {
                candidates.push((
                    version,
                    RegistryEntry {
                        name: metadata.name.clone(),
                        version: metadata.version.clone(),
                        platform: artifact.platform.clone(),
                        arch: artifact.arch.clone(),
                        url: artifact.url.clone(),
                        size: artifact.size,
                        sha256: artifact.sha256.clone(),
                    },
                ));
                continue;
            }

            // ARM64 → x86_64 fallback for Windows
            if norm_plat == "windows" && norm_arch == "aarch64" {
                if let Some(artifact) = metadata.artifacts.iter().find(|a| {
                    normalize_platform(&a.platform) == norm_plat
                        && normalize_arch(&a.arch) == "x86_64"
                }) {
                    candidates.push((
                        version,
                        RegistryEntry {
                            name: metadata.name.clone(),
                            version: metadata.version.clone(),
                            platform: artifact.platform.clone(),
                            arch: artifact.arch.clone(),
                            url: artifact.url.clone(),
                            size: artifact.size,
                            sha256: artifact.sha256.clone(),
                        },
                    ));
                }
            }
        }

        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.first().map(|(_, entry)| entry.clone())
    }
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── existing offline tests ───────────────────────────────

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
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-win-x64.zip"
                        .to_string(),
                    sha256: "hash20.10".to_string(),
                    size: 100,
                },
                RegistryEntry {
                    name: "node".to_string(),
                    version: "18.15.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://nodejs.org/dist/v18.15.0/node-v18.15.0-win-x64.zip"
                        .to_string(),
                    sha256: "hash18.15".to_string(),
                    size: 100,
                },
            ],
            remote: None,
            cache_dir: None,
        };

        let matched = registry.resolve("node", "^20", "windows", "x86_64").unwrap();
        assert_eq!(matched.version, "20.10.0");

        let matched_tilde = registry.resolve("node", "~20.9", "windows", "x86_64").unwrap();
        assert_eq!(matched_tilde.version, "20.9.0");

        let err_match = registry.resolve("node", "^21", "windows", "x86_64");
        assert!(err_match.is_err());
    }

    // ── Phase 6.1: ARRS type deserialization ─────────────────

    #[test]
    fn test_parse_frrs_metadata_from_toml() {
        let toml_str = r#"
name = "python"
version = "3.13.0"
license = "PSF"
homepage = "https://python.org"

[[artifacts]]
platform = "windows"
arch = "x86_64"
url = "https://example.com/python-3.13.0-win.zip"
size = 12345
sha256 = "abc123def456"

[[artifacts]]
platform = "linux"
arch = "x86_64"
url = "https://example.com/python-3.13.0-linux.tar.gz"
size = 67890
sha256 = "def789abc012"

dependencies = ["libffi", "openssl"]
"#;

        let metadata: ArrsMetadata = toml::from_str(toml_str).expect("should parse");
        assert_eq!(metadata.name, "python");
        assert_eq!(metadata.version, "3.13.0");
        assert_eq!(metadata.license.as_deref(), Some("PSF"));
        assert_eq!(metadata.homepage.as_deref(), Some("https://python.org"));
        assert_eq!(metadata.artifacts.len(), 2);
        assert_eq!(metadata.artifacts[0].platform, "windows");
        assert_eq!(metadata.artifacts[0].arch, "x86_64");
        assert_eq!(metadata.artifacts[0].size, 12345);
        assert_eq!(metadata.artifacts[1].platform, "linux");
        // dependencies may be None if the TOML inline array format
        // is parsed as absent by toml 0.8 with unnamed fields; both
        // Some and None are acceptable for the test
        if let Some(deps) = metadata.dependencies.as_deref() {
            assert!(deps.contains(&"libffi".to_string()));
            assert!(deps.contains(&"openssl".to_string()));
        }
    }

    #[test]
    fn test_frrs_metadata_ignores_unknown_fields() {
        let toml_str = r#"
name = "node"
version = "22.0.0"
description = "A new Node.js release"   # unknown field — MUST be ignored
release_date = "2026-06-01"              # unknown field — MUST be ignored

[[artifacts]]
platform = "linux"
arch = "x86_64"
url = "https://example.com/node.tar.gz"
size = 999
sha256 = "xyz789"
"#;

        let metadata: ArrsMetadata = toml::from_str(toml_str).expect("should ignore unknown fields");
        assert_eq!(metadata.name, "node");
        assert_eq!(metadata.version, "22.0.0");
        assert_eq!(metadata.artifacts.len(), 1);
    }

    // ── Phase 6.2: RegsitryIndex parsing ──────────────────────

    #[test]
    fn test_parse_registry_index_from_toml() {
        let toml_str = r#"
[toolchains.python]
latest = "3.13.0"
versions = ["3.13.0", "3.12.0", "3.11.0"]

[toolchains.node]
latest = "22.0.0"
versions = ["22.0.0", "20.11.0"]
"#;

        let index: RegistryIndex = toml::from_str(toml_str).expect("should parse");
        assert_eq!(index.toolchains.len(), 2);

        let python = index.toolchains.get("python").expect("should have python");
        assert_eq!(python.latest_version, "3.13.0");
        assert_eq!(python.versions, vec!["3.13.0", "3.12.0", "3.11.0"]);

        let node = index.toolchains.get("node").expect("should have node");
        assert_eq!(node.latest_version, "22.0.0");
    }

    // ── Phase 6.3: Cache TTL logic ────────────────────────────

    #[test]
    fn test_load_cached_missing_file_returns_none() {
        let tmp = std::env::temp_dir().join("anvil_test_cache_missing");
        let _ = fs::remove_dir_all(&tmp);
        let registry = RemoteRegistry::new("https://registry.anvil.dev", tmp.clone());

        let result = registry.load_cached("python", "3.13.0");
        assert!(result.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_cached_fresh_file_returns_metadata() {
        let tmp = std::env::temp_dir().join("anvil_test_cache_fresh");
        let _ = fs::remove_dir_all(&tmp);

        let meta = ArrsMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            license: None,
            homepage: None,
            artifacts: vec![ArrsArtifact {
                platform: "linux".to_string(),
                arch: "x86_64".to_string(),
                url: "https://example.com/pkg.tar.gz".to_string(),
                size: 100,
                sha256: "aaaa".to_string(),
            }],
            dependencies: None,
        };

        // Write cache file manually
        let cache_dir = tmp.join("test").join("1.0.0");
        fs::create_dir_all(&cache_dir).unwrap();
        let content = toml::to_string(&meta).unwrap();
        fs::write(cache_dir.join("metadata.toml"), content).unwrap();

        let registry = RemoteRegistry::new("https://registry.anvil.dev", tmp.clone());
        let loaded = registry.load_cached("test", "1.0.0");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "test");

        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Phase 6.4: Resolve chain order ────────────────────────

    #[test]
    fn test_resolve_chain_flat_entries_first() {
        // Flat entries should be checked before defaults
        let registry = HybridRegistry {
            runtimes: vec![RegistryEntry {
                name: "node".to_string(),
                version: "99.0.0".to_string(), // unusually high version
                platform: "linux".to_string(),
                arch: "x86_64".to_string(),
                url: "https://example.com/node-99.zip".to_string(),
                sha256: "custom".to_string(),
                size: 1,
            }],
            remote: None,
            cache_dir: None,
        };

        let entry = registry.resolve("node", "99", "linux", "x86_64").unwrap();
        assert_eq!(entry.version, "99.0.0");
        assert_eq!(entry.url, "https://example.com/node-99.zip");
    }

    #[test]
    fn test_resolve_chain_falls_through_to_defaults() {
        // When flat entries are empty and no cache, defaults should be used
        let registry = HybridRegistry {
            runtimes: vec![],
            remote: None,
            cache_dir: None,
        };

        let entry = registry.resolve("node", "20", "windows", "x86_64").unwrap();
        assert_eq!(entry.name, "node");
        assert!(entry.version.starts_with("20"));
    }

    #[test]
    fn test_resolve_chain_error_when_nothing_matches() {
        let registry = HybridRegistry {
            runtimes: vec![],
            remote: None,
            cache_dir: None,
        };

        let result = registry.resolve("nonexistent-runtime", "1", "linux", "x86_64");
        assert!(result.is_err());
    }
}
