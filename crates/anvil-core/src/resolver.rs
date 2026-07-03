use std::collections::HashMap;
use crate::types::{RuntimeLock, EmulationLog};
use crate::registry::{HybridRegistry, normalize_arch, normalize_platform, detect_platform, detect_arch};

pub trait RuntimeProvider: Send + Sync {
    fn name(&self) -> &str;
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String>;
}

pub fn resolve_from_registry(
    name: &str,
    version_req: &str,
    platform: &str,
    arch: &str,
    registry: &HybridRegistry,
) -> Result<RuntimeLock, String> {
    let entry = registry.resolve(name, version_req, platform, arch)?;
    
    let req_arch = normalize_arch(arch);
    let resolved_arch = normalize_arch(&entry.arch);
    let norm_plat = normalize_platform(platform);
    
    let emulation = if norm_plat == "windows" && req_arch == "aarch64" && resolved_arch == "x86_64" {
        Some(EmulationLog {
            requested: "windows-arm64".to_string(),
            installed: "windows-x86_64".to_string(),
            reason: "Native build unavailable".to_string(),
        })
    } else {
        None
    };

    Ok(RuntimeLock {
        name: entry.name,
        version: entry.version,
        platform: entry.platform,
        arch: entry.arch,
        url: entry.url,
        size: entry.size,
        sha256: entry.sha256,
        emulation,
    })
}

pub struct NodeProvider;
impl RuntimeProvider for NodeProvider {
    fn name(&self) -> &str { "node" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct PythonProvider;
impl RuntimeProvider for PythonProvider {
    fn name(&self) -> &str { "python" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct BunProvider;
impl RuntimeProvider for BunProvider {
    fn name(&self) -> &str { "bun" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct GoProvider;
impl RuntimeProvider for GoProvider {
    fn name(&self) -> &str { "go" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct RustProvider;
impl RuntimeProvider for RustProvider {
    fn name(&self) -> &str { "rust" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct LlvmProvider;
impl RuntimeProvider for LlvmProvider {
    fn name(&self) -> &str { "llvm" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct JdkProvider;
impl RuntimeProvider for JdkProvider {
    fn name(&self) -> &str { "jdk" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct Resolver {
    pub providers: HashMap<String, Box<dyn RuntimeProvider>>,
    /// Plugin runtime providers checked as fallback after built-in.
    /// Built-in providers always win on name conflict.
    pub plugin_providers: Vec<Box<dyn RuntimeProvider>>,
}

impl Resolver {
    pub fn new() -> Self {
        let mut providers: HashMap<String, Box<dyn RuntimeProvider>> = HashMap::new();
        providers.insert("node".to_string(), Box::new(NodeProvider));
        providers.insert("python".to_string(), Box::new(PythonProvider));
        providers.insert("bun".to_string(), Box::new(BunProvider));
        providers.insert("go".to_string(), Box::new(GoProvider));
        providers.insert("rust".to_string(), Box::new(RustProvider));
        providers.insert("llvm".to_string(), Box::new(LlvmProvider));
        providers.insert("jdk".to_string(), Box::new(JdkProvider));
        Self {
            providers,
            plugin_providers: Vec::new(),
        }
    }

    pub fn register<P: RuntimeProvider + 'static>(&mut self, provider: P) {
        self.providers.insert(provider.name().to_string(), Box::new(provider));
    }

    /// Adds a plugin runtime provider (built-in wins on name conflict).
    ///
    /// Plugin providers are checked after built-in providers. If a built-in
    /// provider exists with the same name, the plugin provider is skipped.
    pub fn add_plugin_provider(&mut self, provider: Box<dyn RuntimeProvider>) {
        let name = provider.name().to_string();
        if !self.providers.contains_key(&name) {
            self.plugin_providers.push(provider);
        }
    }

    pub fn resolve(&self, name: &str, version: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        // 1. Check built-in providers first
        if let Some(provider) = self.providers.get(name) {
            return provider.resolve(version, platform, arch, registry);
        }
        // 2. Check plugin providers as fallback
        for provider in &self.plugin_providers {
            if provider.name() == name {
                return provider.resolve(version, platform, arch, registry);
            }
        }
        // 3. Final fallback: registry metadata lookup
        resolve_from_registry(name, version, platform, arch, registry)
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

// Preserve stable function interface for backward compatibility
pub fn resolve_runtime_lock(name: &str, version: &str) -> Result<RuntimeLock, String> {
    let registry = HybridRegistry::default_with_internal();
    let platform = detect_platform();
    let arch = detect_arch();
    let resolver = Resolver::new();
    resolver.resolve(name, version, platform, arch, &registry)
}

use crate::environment::RuntimeContextProvider;
use crate::secrets::{ConfigurationProvider, ValueSource, VarMetadata, ResolvedEnvironment, SecretProvider};
use serde::Deserialize;
use sha2::Digest;
use regex::Regex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoctorIssue {
    pub id: String,
    pub severity: String, // "critical" | "warning"
    pub tool: String,     // "config"
    pub message: String,
    pub remediation: String,
}

pub fn toml_value_to_string(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(arr) => {
            let json_arr: Vec<serde_json::Value> = arr.iter().map(toml_value_to_json).collect();
            serde_json::to_string(&json_arr).unwrap_or_default()
        }
        toml::Value::Table(tbl) => {
            let mut json_map = serde_json::Map::new();
            for (key, val) in tbl {
                json_map.insert(key.clone(), toml_value_to_json(val));
            }
            serde_json::to_string(&json_map).unwrap_or_default()
        }
    }
}

fn toml_value_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(arr) => {
            let json_arr = arr.iter().map(toml_value_to_json).collect();
            serde_json::Value::Array(json_arr)
        }
        toml::Value::Table(tbl) => {
            let mut json_map = serde_json::Map::new();
            for (key, val) in tbl {
                json_map.insert(key.clone(), toml_value_to_json(val));
            }
            serde_json::Value::Object(json_map)
        }
    }
}

pub fn interpolate_value(
    value: &str,
    ctx: &dyn RuntimeContextProvider,
    resolved_vars: &HashMap<String, String>,
) -> Result<String, String> {
    let re = Regex::new(r"\$\{([^}]+)\}").map_err(|e| e.to_string())?;
    
    let mut current = value.to_string();
    let mut changed = true;
    let mut iterations = 0;
    
    while changed && iterations < 10 {
        changed = false;
        iterations += 1;
        
        let mut new_str = String::new();
        let mut last_idx = 0;
        
        for cap in re.captures_iter(&current) {
            let m = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str().trim();
            
            new_str.push_str(&current[last_idx..m.start()]);
            
            let resolved = if var_name == "workspace.root" {
                ctx.workspace_root().to_string_lossy().to_string()
            } else if var_name.starts_with("runtime.") && var_name.ends_with(".path") {
                let runtime_name = &var_name["runtime.".len()..var_name.len() - ".path".len()];
                if let Some(path) = ctx.runtime_path(runtime_name) {
                    path.to_string_lossy().to_string()
                } else {
                    return Err(format!("Runtime '{}' not found for interpolation", runtime_name));
                }
            } else if var_name.starts_with("env.") {
                let env_key = &var_name["env.".len()..];
                if let Some(val) = resolved_vars.get(env_key) {
                    val.clone()
                } else {
                    std::env::var(env_key).unwrap_or_default()
                }
            } else {
                if let Some(val) = resolved_vars.get(var_name) {
                    val.clone()
                } else {
                    m.as_str().to_string()
                }
            };
            
            new_str.push_str(&resolved);
            last_idx = m.end();
            changed = true;
        }
        
        new_str.push_str(&current[last_idx..]);
        current = new_str;
    }
    
    Ok(current)
}

pub fn validate_environment(
    vars: &HashMap<String, String>,
    definitions: &HashMap<String, crate::manifest::ConfigDefinition>,
) -> Result<Vec<DoctorIssue>, String> {
    let mut issues = Vec::new();
    
    for (key, def) in definitions {
        let value_opt = vars.get(key);
        
        if def.required && value_opt.is_none() {
            issues.push(DoctorIssue {
                id: format!("config::missing::{}", key),
                severity: "critical".to_string(),
                tool: "config".to_string(),
                message: format!("Required environment variable '{}' is missing", key),
                remediation: format!("Define '{}' in anvil.env, anvil.local.toml, system environment, or secrets.", key),
            });
            continue;
        }
        
        if let Some(value) = value_opt {
            if let Some(ref val_type) = def.val_type {
                match val_type.as_str() {
                    "integer" => {
                        if value.parse::<i64>().is_err() {
                            issues.push(DoctorIssue {
                                id: format!("config::type::{}", key),
                                severity: "critical".to_string(),
                                tool: "config".to_string(),
                                message: format!("Variable '{}' value '{}' is not a valid integer", key, value),
                                remediation: format!("Change '{}' to be a valid integer value.", key),
                            });
                        }
                    }
                    "boolean" => {
                        let lower = value.to_lowercase();
                        if lower != "true" && lower != "false" && lower != "1" && lower != "0" {
                            issues.push(DoctorIssue {
                                id: format!("config::type::{}", key),
                                severity: "critical".to_string(),
                                tool: "config".to_string(),
                                message: format!("Variable '{}' value '{}' is not a valid boolean", key, value),
                                remediation: format!("Change '{}' to be 'true' or 'false'.", key),
                            });
                        }
                    }
                    _ => {}
                }
            }
            
            if let Some(ref pattern_str) = def.pattern {
                match Regex::new(pattern_str) {
                    Ok(re) => {
                        if !re.is_match(value) {
                            issues.push(DoctorIssue {
                                id: format!("config::pattern::{}", key),
                                severity: "critical".to_string(),
                                tool: "config".to_string(),
                                message: format!("Variable '{}' value does not match pattern '{}'", key, pattern_str),
                                remediation: format!("Update '{}' to match the regex: {}", key, pattern_str),
                            });
                        }
                    }
                    Err(e) => {
                        issues.push(DoctorIssue {
                            id: format!("config::schema::{}", key),
                            severity: "warning".to_string(),
                            tool: "config".to_string(),
                            message: format!("Invalid pattern regex '{}' defined for '{}': {}", pattern_str, key, e),
                            remediation: "Correct the regex pattern in the anvil.toml config definitions.".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(issues)
}

/// Resolves the configuration environment using the standard stack (no plugin providers).
/// Delegates to `resolve_environment_with_plugins` with an empty plugin list.
pub fn resolve_environment(
    ctx: &dyn RuntimeContextProvider,
    cli_overrides: &HashMap<String, String>,
    active_profile: Option<&str>,
) -> Result<ResolvedEnvironment, String> {
    resolve_environment_with_plugins(ctx, cli_overrides, active_profile, &[])
}

/// Full version of `resolve_environment` that also accepts plugin configuration providers.
///
/// Plugin providers are inserted at **level 2.5** in the resolution stack:
/// 1. CLI Flags / `ANVIL_VAR_<KEY>`
/// 2. System environment overrides (ANVIL_VAR_<KEY>)
/// 3. Local Developer Overrides (anvil.local.toml)
/// 4. **Plugin Configuration Providers** (new)
/// 5. Secrets Providers (anvil.secrets)
/// 6. Environment File (anvil.env)
/// 7. Profile Overlays
/// 8. Defaults / Manifest
pub fn resolve_environment_with_plugins(
    ctx: &dyn RuntimeContextProvider,
    cli_overrides: &HashMap<String, String>,
    active_profile: Option<&str>,
    plugin_config_providers: &[Box<dyn ConfigurationProvider>],
) -> Result<ResolvedEnvironment, String> {
    let mut vars = HashMap::new();
    let mut metadata = HashMap::new();

    let toml_path = ctx.workspace_root().join("anvil.toml");
    let config = if toml_path.exists() {
        crate::manifest::load_config(&toml_path).ok()
    } else {
        None
    };

    let workspace_id = config
        .as_ref()
        .and_then(|c| c.workspace_id.clone())
        .unwrap_or_else(|| {
            let hash_bytes = sha2::Sha256::digest(ctx.workspace_root().to_string_lossy().as_bytes());
            hex::encode(&hash_bytes[..8])
        });

    // Level 7: Defaults
    if let Some(ref cfg) = config {
        if let Some(ref config_sec) = cfg.config {
            for (key, def) in &config_sec.definitions {
                if let Some(ref default_val) = def.default {
                    let val_str = toml_value_to_string(default_val);
                    vars.insert(key.clone(), val_str);
                    metadata.insert(key.clone(), VarMetadata {
                        key: key.clone(),
                        source: ValueSource::DefaultManifest,
                    });
                }
            }
        }
    }

    // Level 6: Profile Overlays
    if let Some(profile_name) = active_profile {
        if let Some(ref cfg) = config {
            if let Some(ref profiles) = cfg.profile {
                if let Some(profile_sec) = profiles.get(profile_name) {
                    for (key, val) in &profile_sec.env {
                        let val_str = toml_value_to_string(val);
                        vars.insert(key.clone(), val_str);
                        metadata.insert(key.clone(), VarMetadata {
                            key: key.clone(),
                            source: ValueSource::ProfileOverlay(profile_name.to_string()),
                        });
                    }
                }
            }
        }
    }

    // Level 5: Env File (anvil.env)
    if let Some(env_path) = crate::environment::find_anvil_env(ctx.workspace_root()) {
        if let Ok(env_vars) = crate::environment::parse_env_file(&env_path) {
            for (key, val) in env_vars {
                vars.insert(key.clone(), val);
                metadata.insert(key.clone(), VarMetadata {
                    key: key.clone(),
                    source: ValueSource::EnvFile,
                });
            }
        }
    }

    // Level 4: Secrets Providers
    let secrets_path = ctx.workspace_root().join("anvil.secrets");
    if secrets_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&secrets_path) {
            #[derive(Deserialize)]
            struct SecretsManifest {
                secrets: HashMap<String, SecretConfig>,
            }
            #[derive(Deserialize)]
            struct SecretConfig {
                provider: String,
                key: Option<String>,
                #[serde(rename = "secret_id")]
                secret_id: Option<String>,
            }

            if let Ok(secrets_manifest) = toml::from_str::<SecretsManifest>(&content) {
                for (key, secret_cfg) in secrets_manifest.secrets {
                    let resolved_secret = if secret_cfg.provider == "keyring" {
                        let provider = crate::secrets::KeyringSecretProvider::new(&workspace_id);
                        let lookup_key = secret_cfg.key.as_deref().or(secret_cfg.secret_id.as_deref()).unwrap_or(&key);
                        provider.get(lookup_key).ok().flatten()
                    } else if secret_cfg.provider == "file" {
                        let provider = crate::secrets::FallbackSecretProvider::new(
                            &workspace_id,
                            ctx.workspace_root().join(".anvil").join("secrets.enc")
                        );
                        let lookup_key = secret_cfg.key.as_deref().or(secret_cfg.secret_id.as_deref()).unwrap_or(&key);
                        provider.get(lookup_key).ok().flatten()
                    } else {
                        None
                    };

                    if let Some(secret_val) = resolved_secret {
                        vars.insert(key.clone(), secret_val);
                        metadata.insert(key.clone(), VarMetadata {
                            key: key.clone(),
                            source: ValueSource::SecretProvider(secret_cfg.provider.clone()),
                        });
                    }
                }
            }
        }
    }

    // Level 2.5: Plugin Configuration Providers
    // Inserted between Local Overrides (L3) and Secrets (L4).
    // Plugin values are overridden by local overrides but override secrets.
    for provider in plugin_config_providers {
        if let Ok(plugin_vars) = provider.load(ctx) {
            for (key, val) in plugin_vars {
                vars.insert(key.clone(), val);
                metadata.insert(key.clone(), VarMetadata {
                    key: key.clone(),
                    source: ValueSource::SecretProvider(provider.name().to_string()),
                });
            }
        }
    }

    // Level 3: Local Developer Overrides (anvil.local.toml)
    let local_path = ctx.workspace_root().join("anvil.local.toml");
    if local_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&local_path) {
            #[derive(Deserialize)]
            struct LocalManifest {
                env: Option<HashMap<String, toml::Value>>,
            }
            if let Ok(local_manifest) = toml::from_str::<LocalManifest>(&content) {
                if let Some(env_block) = local_manifest.env {
                    for (key, val) in env_block {
                        let val_str = toml_value_to_string(&val);
                        vars.insert(key.clone(), val_str);
                        metadata.insert(key.clone(), VarMetadata {
                            key: key.clone(),
                            source: ValueSource::LocalOverride,
                        });
                    }
                }
            }
        }
    }

    // Level 2: System Environment Overrides (ANVIL_VAR_<KEY>)
    for (k, v) in std::env::vars() {
        if k.starts_with("ANVIL_VAR_") {
            let var_key = &k["ANVIL_VAR_".len()..];
            vars.insert(var_key.to_string(), v);
            metadata.insert(var_key.to_string(), VarMetadata {
                key: var_key.to_string(),
                source: ValueSource::SystemEnv,
            });
        }
    }

    // Level 1: CLI Overrides
    for (k, v) in cli_overrides {
        vars.insert(k.clone(), v.clone());
        metadata.insert(k.clone(), VarMetadata {
            key: k.clone(),
            source: ValueSource::CliOverride,
        });
    }

    // Interpolate values
    let vars_clone = vars.clone();
    for (_k, v) in vars.iter_mut() {
        if let Ok(interpolated) = interpolate_value(v, ctx, &vars_clone) {
            *v = interpolated;
        }
    }

    Ok(ResolvedEnvironment { vars, metadata })
}

#[cfg(test)]
mod resolver_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct MockRuntimeContextProvider {
        workspace_root: PathBuf,
        runtimes: HashMap<String, PathBuf>,
    }

    impl RuntimeContextProvider for MockRuntimeContextProvider {
        fn workspace_root(&self) -> &Path {
            &self.workspace_root
        }
        fn runtime_path(&self, name: &str) -> Option<PathBuf> {
            self.runtimes.get(name).cloned()
        }
    }

    #[test]
    fn test_interpolation() {
        let ctx = MockRuntimeContextProvider {
            workspace_root: PathBuf::from("/my/workspace"),
            runtimes: [("python".to_string(), PathBuf::from("/bin/python"))].into_iter().collect(),
        };

        let mut resolved_vars = HashMap::new();
        resolved_vars.insert("DATABASE_URL".to_string(), "postgres://localhost".to_string());

        let input = "Root is ${workspace.root}, python at ${runtime.python.path}, and url is ${DATABASE_URL}";
        let output = interpolate_value(input, &ctx, &resolved_vars).unwrap();
        assert_eq!(output, "Root is /my/workspace, python at /bin/python, and url is postgres://localhost");
    }

    /// Verifies plugin ConfigurationProvider level 2.5 precedence ordering.
    /// Stack: CLI overrides > System env > Local overrides > **Plugin** > Secrets > Env file > Profile > Defaults
    #[test]
    fn test_plugin_config_provider_precedence() {
        use crate::secrets::ConfigurationProvider;

        struct MockConfigProvider;
        impl ConfigurationProvider for MockConfigProvider {
            fn name(&self) -> &str { "mock-plugin-config" }
            fn load(&self, _ctx: &dyn RuntimeContextProvider) -> Result<HashMap<String, String>, String> {
                let mut map = HashMap::new();
                map.insert("PLUGIN_VAR".to_string(), "from-plugin".to_string());
                map.insert("OVERRIDE_TEST".to_string(), "plugin-value".to_string());
                Ok(map)
            }
        }

        struct OverrideConfigProvider;
        impl ConfigurationProvider for OverrideConfigProvider {
            fn name(&self) -> &str { "override-plugin-config" }
            fn load(&self, _ctx: &dyn RuntimeContextProvider) -> Result<HashMap<String, String>, String> {
                let mut map = HashMap::new();
                map.insert("OVERRIDE_TEST".to_string(), "override-plugin".to_string());
                Ok(map)
            }
        }

        let ctx = MockRuntimeContextProvider {
            workspace_root: PathBuf::from("/tmp/anvil_test_plugin_config"),
            runtimes: HashMap::new(),
        };

        let plugin_providers: Vec<Box<dyn ConfigurationProvider>> = vec![
            Box::new(MockConfigProvider),
            Box::new(OverrideConfigProvider),
        ];

        let result = resolve_environment_with_plugins(&ctx, &HashMap::new(), None, &plugin_providers).unwrap();
        assert_eq!(result.vars.get("PLUGIN_VAR").unwrap(), "from-plugin",
            "Plugin provider value should be present");

        // When two plugin providers set the same key, the last one wins (insert order)
        assert_eq!(result.vars.get("OVERRIDE_TEST").unwrap(), "override-plugin",
            "Last plugin provider should override earlier ones");

        // Test that CLI overrides beat plugin providers
        let mut cli_overrides = HashMap::new();
        cli_overrides.insert("PLUGIN_VAR".to_string(), "from-cli".to_string());
        let result2 = resolve_environment_with_plugins(&ctx, &cli_overrides, None, &plugin_providers).unwrap();
        assert_eq!(result2.vars.get("PLUGIN_VAR").unwrap(), "from-cli",
            "CLI override should beat plugin provider value");
    }

    #[test]
    fn test_validation() {
        let mut vars = HashMap::new();
        vars.insert("DATABASE_URL".to_string(), "postgres://localhost".to_string());
        vars.insert("MAX_CONNECTIONS".to_string(), "invalid-int".to_string());

        let mut definitions = HashMap::new();
        definitions.insert("DATABASE_URL".to_string(), crate::manifest::ConfigDefinition {
            val_type: Some("string".to_string()),
            required: true,
            default: None,
            pattern: Some("^postgres://.*".to_string()),
            description: None,
            secret: false,
        });
        definitions.insert("MAX_CONNECTIONS".to_string(), crate::manifest::ConfigDefinition {
            val_type: Some("integer".to_string()),
            required: true,
            default: None,
            pattern: None,
            description: None,
            secret: false,
        });
        definitions.insert("MISSING_VAR".to_string(), crate::manifest::ConfigDefinition {
            val_type: Some("string".to_string()),
            required: true,
            default: None,
            pattern: None,
            description: None,
            secret: false,
        });

        let issues = validate_environment(&vars, &definitions).unwrap();
        assert_eq!(issues.len(), 2);
        
        let missing_issue = issues.iter().find(|i| i.id.contains("missing")).unwrap();
        assert_eq!(missing_issue.id, "config::missing::MISSING_VAR");

        let type_issue = issues.iter().find(|i| i.id.contains("type")).unwrap();
        assert_eq!(type_issue.id, "config::type::MAX_CONNECTIONS");
    }

    // ── New Runtime Providers ─────────────────────────────────

    #[test]
    fn test_llvm_provider_name() {
        let provider = LlvmProvider;
        assert_eq!(provider.name(), "llvm");
    }

    #[test]
    fn test_jdk_provider_name() {
        let provider = JdkProvider;
        assert_eq!(provider.name(), "jdk");
    }

    #[test]
    fn test_llvm_registry_resolve_all_platforms() {
        use crate::registry::HybridRegistry;
        let registry = HybridRegistry::default_with_internal();

        let cases = vec![
            ("windows", "x86_64", "18.1.0"),
            ("macos", "x86_64", "18.1.0"),
            ("macos", "aarch64", "18.1.0"),
            ("linux", "x86_64", "18.1.0"),
            ("linux", "aarch64", "18.1.0"),
        ];
        for (platform, arch, version) in cases {
            let result = registry.resolve("llvm", version, platform, arch);
            assert!(result.is_ok(), "llvm {} {}/{} should resolve", version, platform, arch);
            let entry = result.unwrap();
            assert_eq!(entry.name, "llvm");
            assert_eq!(entry.version, version);
        }
    }

    #[test]
    fn test_jdk_registry_resolve_all_platforms() {
        use crate::registry::HybridRegistry;
        let registry = HybridRegistry::default_with_internal();

        let cases = vec![
            ("windows", "x86_64", "21.0.2"),
            ("macos", "x86_64", "21.0.2"),
            ("macos", "aarch64", "21.0.2"),
            ("linux", "x86_64", "21.0.2"),
            ("linux", "aarch64", "21.0.2"),
        ];
        for (platform, arch, version) in cases {
            let result = registry.resolve("jdk", version, platform, arch);
            assert!(result.is_ok(), "jdk {} {}/{} should resolve", version, platform, arch);
            let entry = result.unwrap();
            assert_eq!(entry.name, "jdk");
            assert_eq!(entry.version, version);
        }
    }

    #[test]
    fn test_default_with_internal_contains_llvm_and_jdk() {
        use crate::registry::HybridRegistry;
        let registry = HybridRegistry::default_with_internal();
        let llvm_entries: Vec<_> = registry.runtimes.iter().filter(|e| e.name == "llvm").collect();
        let jdk_entries: Vec<_> = registry.runtimes.iter().filter(|e| e.name == "jdk").collect();
        assert!(!llvm_entries.is_empty(), "default_with_internal() should contain llvm entries");
        assert!(!jdk_entries.is_empty(), "default_with_internal() should contain jdk entries");
    }

    #[test]
    fn test_resolve_nonexistent_runtime_returns_error() {
        use crate::registry::HybridRegistry;
        let registry = HybridRegistry::default_with_internal();
        let result = registry.resolve("nonexistent-runtime", "1.0.0", "linux", "x86_64");
        assert!(result.is_err(), "resolve of nonexistent runtime should return Err");
    }
}

