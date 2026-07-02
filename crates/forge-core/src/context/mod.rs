use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::secrets::SecretProvider;

// --- Traits ---

pub trait ContextProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String>;
}

pub trait ContextExporter: Send + Sync {
    fn name(&self) -> &'static str;
    fn export(&self, context: &ForgeContext) -> Result<String, String>;
}

pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn adapt(&self, context: &ForgeContext, exporter: &dyn ContextExporter) -> Result<String, String>;
}

// --- Options and Context schemas ---

#[derive(Debug, Clone, Default)]
pub struct ContextOptions {
    pub scopes: Vec<String>,
    pub excludes: Vec<String>,
    pub workspace_root: PathBuf,
    pub cache_dir: PathBuf,
    pub active_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForgeContext {
    pub schema_version: String,
    pub runtimes: serde_json::Value,
    pub config: serde_json::Value,
    pub diagnostics: serde_json::Value,
    pub workspace: serde_json::Value,
    pub environment: serde_json::Value,
    pub secrets_metadata: serde_json::Value,
}

// --- FCP Handshake structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeCapabilities {
    pub scopes: Vec<String>,
    pub exporters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeParams {
    pub version: String,
    pub capabilities: HandshakeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcpHandshakeRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: HandshakeParams,
    pub id: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub version: String,
    pub negotiated_capabilities: HandshakeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcpHandshakeResponse {
    pub jsonrpc: String,
    pub result: HandshakeResult,
    pub id: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcpHandshake {
    pub request: FcpHandshakeRequest,
}

// --- ContextEngine ---

pub struct ContextEngine {
    pub providers: Vec<Arc<dyn ContextProvider>>,
    pub exporters: Vec<Arc<dyn ContextExporter>>,
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextEngine {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            exporters: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn ContextProvider>) {
        self.providers.push(provider);
    }

    /// Registers plugin context providers alongside built-in providers.
    pub fn register_plugin_providers(&mut self, providers: Vec<Arc<dyn ContextProvider>>) {
        self.providers.extend(providers);
    }

    /// Registers a plugin context exporter, rejecting duplicate names
    /// with a warning. The first registered exporter for a given name
    /// retains precedence (built-in exporters registered first).
    pub fn register_plugin_exporter(
        &mut self,
        exporter: Arc<dyn ContextExporter>,
    ) -> Result<(), String> {
        let name = exporter.name().to_string();
        if self.exporters.iter().any(|e| e.name() == name) {
            return Err(format!(
                "Duplicate context exporter '{}': a plugin exporter with this name is already registered",
                name
            ));
        }
        self.exporters.push(exporter);
        Ok(())
    }

    /// Looks up a context exporter by name. Checks built-in exporters
    /// first, then plugin exporters.
    pub fn get_exporter(&self, name: &str) -> Option<Arc<dyn ContextExporter>> {
        self.exporters.iter().find(|e| e.name() == name).cloned()
    }

    pub fn handshake(&self, req_json: &str) -> Result<String, String> {
        let req: FcpHandshakeRequest = serde_json::from_str(req_json)
            .map_err(|e| format!("Invalid JSON-RPC request: {}", e))?;

        if req.jsonrpc != "2.0" {
            return Err("Unsupported JSON-RPC version".to_string());
        }
        if req.method != "fcp.handshake" {
            return Err("Unsupported method. Expected fcp.handshake".to_string());
        }

        // Negotiate version (we support "1.0.0")
        let negotiated_version = "1.0.0".to_string();

        // Supported capability matching
        let supported_scopes = vec![
            "runtime".to_string(),
            "configuration".to_string(),
            "diagnostics".to_string(),
            "workspace".to_string(),
            "environment".to_string(),
            "secrets".to_string(),
        ];
        let supported_exporters = vec![
            "json".to_string(),
            "markdown".to_string(),
            "mcp".to_string(),
        ];

        let mut negotiated_scopes = Vec::new();
        for scope in &req.params.capabilities.scopes {
            let normalized = match scope.as_str() {
                "runtimes" => "runtime",
                "config" => "configuration",
                s => s,
            };
            if supported_scopes.contains(&normalized.to_string()) {
                negotiated_scopes.push(normalized.to_string());
            }
        }

        let mut negotiated_exporters = Vec::new();
        for exporter in &req.params.capabilities.exporters {
            if supported_exporters.contains(exporter) {
                negotiated_exporters.push(exporter.clone());
            }
        }

        let response = FcpHandshakeResponse {
            jsonrpc: "2.0".to_string(),
            result: HandshakeResult {
                version: negotiated_version,
                negotiated_capabilities: HandshakeCapabilities {
                    scopes: negotiated_scopes,
                    exporters: negotiated_exporters,
                },
            },
            id: req.id,
        };

        serde_json::to_string(&response)
            .map_err(|e| format!("Failed to serialize response: {}", e))
    }

    pub async fn query(&self, options: &ContextOptions) -> Result<ForgeContext, String> {
        let mut tasks = Vec::new();

        for provider in &self.providers {
            let name = provider.name();
            let matches_scope = if options.scopes.is_empty() {
                true
            } else {
                options.scopes.iter().any(|s| {
                    let s_clean = s.to_lowercase();
                    name.to_lowercase() == s_clean
                        || (name == "runtime" && s_clean == "runtimes")
                        || (name == "configuration" && s_clean == "config")
                        || (name == "secrets" && s_clean == "secrets_metadata")
                })
            };

            if !matches_scope {
                continue;
            }

            let provider = provider.clone();
            let opt = options.clone();

            let handle = tokio::task::spawn_blocking(move || {
                provider.collect(&opt)
            });

            tasks.push((name, handle));
        }

        let mut runtimes = serde_json::Value::Null;
        let mut config = serde_json::Value::Null;
        let mut diagnostics = serde_json::Value::Null;
        let mut workspace = serde_json::Value::Null;
        let mut environment = serde_json::Value::Null;
        let mut secrets_metadata = serde_json::Value::Null;

        for (name, handle) in tasks {
            match tokio::time::timeout(std::time::Duration::from_millis(5000), handle).await {
                Ok(Ok(res)) => {
                    let val = match res {
                        Ok(v) => v,
                        Err(e) => serde_json::json!({ "error": e }),
                    };
                    match name {
                        "runtime" => runtimes = val,
                        "configuration" => config = val,
                        "diagnostics" => diagnostics = val,
                        "workspace" => workspace = val,
                        "environment" => environment = val,
                        "secrets" => secrets_metadata = val,
                        _ => {}
                    }
                }
                Ok(Err(_join_err)) => {
                    let val = serde_json::json!({ "error": "Provider task panicked" });
                    match name {
                        "runtime" => runtimes = val,
                        "configuration" => config = val,
                        "diagnostics" => diagnostics = val,
                        "workspace" => workspace = val,
                        "environment" => environment = val,
                        "secrets" => secrets_metadata = val,
                        _ => {}
                    }
                }
                Err(_) => {
                    let val = serde_json::json!({ "error": "Provider query timed out after 5000ms" });
                    match name {
                        "runtime" => runtimes = val,
                        "configuration" => config = val,
                        "diagnostics" => diagnostics = val,
                        "workspace" => workspace = val,
                        "environment" => environment = val,
                        "secrets" => secrets_metadata = val,
                        _ => {}
                    }
                }
            }
        }

        Ok(ForgeContext {
            schema_version: "1.0.0".to_string(),
            runtimes,
            config,
            diagnostics,
            workspace,
            environment,
            secrets_metadata,
        })
    }
}

// --- Concrete Providers ---

pub struct RuntimeProviderImpl;

impl ContextProvider for RuntimeProviderImpl {
    fn name(&self) -> &'static str {
        "runtime"
    }

    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String> {
        let lockfile_path = options.workspace_root.join("forge.lock");
        if lockfile_path.exists() {
            let lockfile = crate::lock::load_lockfile(&lockfile_path)?;
            let mut list = Vec::new();
            for rt in lockfile.runtimes {
                list.push(serde_json::json!({
                    "name": rt.name,
                    "version": rt.version,
                    "platform": rt.platform,
                    "arch": rt.arch,
                    "url": rt.url,
                    "size": rt.size,
                    "sha256": rt.sha256,
                }));
            }
            Ok(serde_json::json!({ "runtimes": list }))
        } else {
            Ok(serde_json::json!({ "runtimes": [] }))
        }
    }
}

pub struct ConfigurationProviderImpl;

impl ContextProvider for ConfigurationProviderImpl {
    fn name(&self) -> &'static str {
        "configuration"
    }

    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String> {
        let toml_path = options.workspace_root.join("forge.toml");
        if toml_path.exists() {
            let config = crate::manifest::load_config(&toml_path)?;
            let active_profile = options.active_profile.clone().unwrap_or_else(|| "default".to_string());
            
            let mut payload = serde_json::json!({
                "workspace_id": config.workspace_id,
                "runtimes": config.runtimes,
                "active_profile": active_profile,
            });

            if let Some(config_sec) = config.config {
                payload["definitions"] = serde_json::to_value(&config_sec.definitions).unwrap_or(serde_json::Value::Null);
            }

            if let Some(profiles) = config.profile {
                if let Some(prof_sec) = profiles.get(&active_profile) {
                    payload["active_profile_env"] = serde_json::to_value(&prof_sec.env).unwrap_or(serde_json::Value::Null);
                }
            }

            Ok(payload)
        } else {
            Ok(serde_json::json!({}))
        }
    }
}

pub struct DiagnosticsProviderImpl;

impl ContextProvider for DiagnosticsProviderImpl {
    fn name(&self) -> &'static str {
        "diagnostics"
    }

    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String> {
        let diag_ctx = crate::diagnostics::DiagnosticContext {
            workspace_root: options.workspace_root.clone(),
            cache_dir: options.cache_dir.clone(),
            mode: crate::diagnostics::DiagnosticMode::Fast,
            active_profile: options.active_profile.clone(),
        };
        let engine = crate::diagnostics::DiagnosticEngine::new();
        
        let report = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(async { engine.run(&diag_ctx).await }),
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to build diagnostics tokio runtime: {}", e))?;
                rt.block_on(async { engine.run(&diag_ctx).await })
            }
        };

        // Take last 50 findings
        let mut findings = report.findings;
        if findings.len() > 50 {
            findings.truncate(50);
        }

        Ok(serde_json::json!({
            "timestamp": report.timestamp,
            "mode": format!("{:?}", report.mode),
            "health_score": report.health_score,
            "findings": findings,
            "elapsed_ms": report.elapsed_ms,
        }))
    }
}

pub struct WorkspaceProviderImpl;

struct CrawlEntry {
    path: PathBuf,
    depth: usize,
}

fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        matches!(
            ext_lower.as_str(),
            "exe" | "dll" | "so" | "dylib" | "bin" | "zip" | "tar" | "gz" | "xz" | "rar" | "7z" |
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "pdf" | "mp3" | "mp4" | "wav" | "ogg" |
            "iso" | "dmg" | "class" | "pyc" | "o" | "a" | "lib" | "db" | "sqlite"
        )
    } else {
        false
    }
}

impl ContextProvider for WorkspaceProviderImpl {
    fn name(&self) -> &'static str {
        "workspace"
    }

    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String> {
        let mut stack = vec![CrawlEntry {
            path: options.workspace_root.clone(),
            depth: 0,
        }];
        let mut files = Vec::new();
        let mut file_count = 0;

        let mut gitignore_patterns = Vec::new();
        if let Ok(content) = std::fs::read_to_string(options.workspace_root.join(".gitignore")) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    gitignore_patterns.push(trimmed.to_string());
                }
            }
        }

        while let Some(current) = stack.pop() {
            if current.depth > 5 {
                continue;
            }

            let file_name = current.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name == ".git" || file_name == "node_modules" || file_name == "target" || file_name == ".forge" {
                continue;
            }

            let rel_path = match current.path.strip_prefix(&options.workspace_root) {
                Ok(p) => p.to_string_lossy().replace('\\', "/"),
                Err(_) => current.path.to_string_lossy().replace('\\', "/"),
            };

            if !rel_path.is_empty() {
                let mut gitignored = false;
                for pattern in &gitignore_patterns {
                    let pat = pattern.trim_end_matches('/');
                    if rel_path.split('/').any(|segment| segment == pat) || rel_path.contains(pat) {
                        gitignored = true;
                        break;
                    }
                }
                if gitignored {
                    continue;
                }

                let mut excluded = false;
                for exclude in &options.excludes {
                    if rel_path.contains(exclude) || file_name == exclude {
                        excluded = true;
                        break;
                    }
                }
                if excluded {
                    continue;
                }
            }

            if current.path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&current.path) {
                    let mut dir_entries = Vec::new();
                    for entry in entries {
                        if let Ok(entry) = entry {
                            dir_entries.push(entry.path());
                        }
                    }
                    dir_entries.sort_by(|a, b| b.cmp(a));
                    for path in dir_entries {
                        let is_d = path.is_dir();
                        stack.push(CrawlEntry {
                            path,
                            depth: if is_d { current.depth + 1 } else { current.depth },
                        });
                    }
                }
            } else if current.path.is_file() {
                if is_binary_file(&current.path) {
                    continue;
                }

                if file_count >= 1000 {
                    break;
                }

                if let Ok(metadata) = std::fs::metadata(&current.path) {
                    files.push(serde_json::json!({
                        "path": rel_path,
                        "size": metadata.len(),
                        "modified": metadata.modified().ok()
                            .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    }));
                    file_count += 1;
                }
            }
        }

        Ok(serde_json::Value::Array(files))
    }
}

pub struct EnvironmentProviderImpl;

impl ContextProvider for EnvironmentProviderImpl {
    fn name(&self) -> &'static str {
        "environment"
    }

    fn collect(&self, _options: &ContextOptions) -> Result<serde_json::Value, String> {
        let mut env_map = serde_json::Map::new();
        for (k, v) in std::env::vars() {
            if crate::environment::is_secret(&k) {
                env_map.insert(k, serde_json::Value::String("[MASKED]".to_string()));
            } else {
                env_map.insert(k, serde_json::Value::String(v));
            }
        }
        Ok(serde_json::Value::Object(env_map))
    }
}

pub struct SecretsProviderImpl;

impl ContextProvider for SecretsProviderImpl {
    fn name(&self) -> &'static str {
        "secrets"
    }

    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String> {
        let mut secrets_map = serde_json::Map::new();

        // 1. Check for Keyring secret keys
        let toml_path = options.workspace_root.join("forge.toml");
        if toml_path.exists() {
            if let Ok(config) = crate::manifest::load_config(&toml_path) {
                if let Some(ref ws_id) = config.workspace_id {
                    let keyring_provider = crate::secrets::KeyringSecretProvider::new(ws_id);
                    if let Ok(keys) = keyring_provider.list() {
                        for key in keys {
                            secrets_map.insert(key, serde_json::json!({
                                "source": "keyring",
                                "value": "[MASKED]"
                            }));
                        }
                    }
                }
            }
        }

        // 2. Check for secret keys in active environment variables
        for (k, _) in std::env::vars() {
            if crate::environment::is_secret(&k) {
                if !secrets_map.contains_key(&k) {
                    secrets_map.insert(k, serde_json::json!({
                        "source": "environment",
                        "value": "[MASKED]"
                    }));
                }
            }
        }

        Ok(serde_json::Value::Object(secrets_map))
    }
}

// --- Exporters ---

pub struct JsonExporter {
    pub pretty: bool,
}

impl ContextExporter for JsonExporter {
    fn name(&self) -> &'static str {
        "json"
    }

    fn export(&self, context: &ForgeContext) -> Result<String, String> {
        let val = serde_json::to_value(context)
            .map_err(|e| format!("Failed to serialize context: {}", e))?;
        if self.pretty {
            serde_json::to_string_pretty(&val)
                .map_err(|e| format!("Failed to pretty print context: {}", e))
        } else {
            serde_json::to_string(&val)
                .map_err(|e| format!("Failed to export minified context: {}", e))
        }
    }
}

pub struct MarkdownExporter;

impl ContextExporter for MarkdownExporter {
    fn name(&self) -> &'static str {
        "markdown"
    }

    fn export(&self, context: &ForgeContext) -> Result<String, String> {
        let mut md = String::new();
        md.push_str("# Forge Context Summary\n\n");

        // Runtimes
        md.push_str("## Runtimes\n");
        if let Some(runtimes_arr) = context.runtimes.get("runtimes").and_then(|r| r.as_array()) {
            if runtimes_arr.is_empty() {
                md.push_str("No active runtimes found.\n\n");
            } else {
                md.push_str("| Name | Version | Platform | Arch |\n");
                md.push_str("| --- | --- | --- | --- |\n");
                for rt in runtimes_arr {
                    let name = rt.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                    let version = rt.get("version").and_then(|v| v.as_str()).unwrap_or("-");
                    let platform = rt.get("platform").and_then(|v| v.as_str()).unwrap_or("-");
                    let arch = rt.get("arch").and_then(|v| v.as_str()).unwrap_or("-");
                    md.push_str(&format!("| {} | {} | {} | {} |\n", name, version, platform, arch));
                }
                md.push_str("\n");
            }
        } else {
            md.push_str("No runtime information collected.\n\n");
        }

        // Configuration
        md.push_str("## Configuration\n");
        if context.config.is_object() {
            md.push_str("| Key | Value |\n");
            md.push_str("| --- | --- |\n");
            if let Some(obj) = context.config.as_object() {
                for (k, v) in obj {
                    let val_str = serde_json::to_string(v).unwrap_or_default();
                    md.push_str(&format!("| {} | {} |\n", k, val_str));
                }
            }
            md.push_str("\n");
        } else {
            md.push_str("No configuration collected.\n\n");
        }

        // Diagnostics
        md.push_str("## Diagnostics\n");
        if context.diagnostics.is_object() {
            let score = context.diagnostics.get("health_score").and_then(|v| v.as_i64()).unwrap_or(0);
            md.push_str(&format!("- **Health Score**: {}\n", score));
            if let Some(findings) = context.diagnostics.get("findings").and_then(|f| f.as_array()) {
                if findings.is_empty() {
                    md.push_str("- **Findings**: Healthy (No issues)\n");
                } else {
                    md.push_str("- **Findings**:\n");
                    for f in findings {
                        let id = f.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let severity = f.get("severity").and_then(|v| v.as_str()).unwrap_or("INFO");
                        let message = f.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        md.push_str(&format!("  - [{}] {}: {}\n", severity, id, message));
                    }
                }
            }
            md.push_str("\n");
        } else {
            md.push_str("No diagnostics collected.\n\n");
        }

        // Workspace
        md.push_str("## Workspace Tree\n");
        if let Some(files) = context.workspace.as_array() {
            if files.is_empty() {
                md.push_str("Empty workspace.\n\n");
            } else {
                md.push_str("```\n");
                for f in files {
                    if let Some(path) = f.get("path").and_then(|p| p.as_str()) {
                        md.push_str(&format!("{}\n", path));
                    }
                }
                md.push_str("```\n\n");
            }
        } else {
            md.push_str("No workspace files collected.\n\n");
        }

        // Environment
        md.push_str("## Environment\n");
        if let Some(obj) = context.environment.as_object() {
            if obj.is_empty() {
                md.push_str("No environment variables.\n\n");
            } else {
                md.push_str("| Key | Value |\n");
                md.push_str("| --- | --- |\n");
                for (k, v) in obj {
                    let val_str = v.as_str().unwrap_or("");
                    md.push_str(&format!("| {} | {} |\n", k, val_str));
                }
                md.push_str("\n");
            }
        } else {
            md.push_str("No environment collected.\n\n");
        }

        // Secrets
        md.push_str("## Secrets\n");
        if let Some(obj) = context.secrets_metadata.as_object() {
            if obj.is_empty() {
                md.push_str("No secrets registered.\n\n");
            } else {
                md.push_str("| Key | Source |\n");
                md.push_str("| --- | --- |\n");
                for (k, v) in obj {
                    let src = v.get("source").and_then(|s| s.as_str()).unwrap_or("unknown");
                    md.push_str(&format!("| {} | {} |\n", k, src));
                }
                md.push_str("\n");
            }
        } else {
            md.push_str("No secrets collected.\n\n");
        }

        Ok(md)
    }
}

pub struct McpExporter;

impl ContextExporter for McpExporter {
    fn name(&self) -> &'static str {
        "mcp"
    }

    fn export(&self, context: &ForgeContext) -> Result<String, String> {
        let serialized = serde_json::to_string(context)
            .map_err(|e| format!("Failed to serialize context: {}", e))?;
        let mcp_envelope = serde_json::json!({
            "contents": [
                {
                    "uri": "forge://context/active",
                    "mimeType": "application/json",
                    "text": serialized
                }
            ]
        });
        serde_json::to_string(&mcp_envelope)
            .map_err(|e| format!("Failed to serialize MCP wrapper: {}", e))
    }
}

// --- Agent Adapters ---

pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn adapt(&self, context: &ForgeContext, _exporter: &dyn ContextExporter) -> Result<String, String> {
        let mut xml = String::new();
        xml.push_str("<forge_context>");
        
        xml.push_str("<runtimes>");
        if let Some(runtimes_arr) = context.runtimes.get("runtimes").and_then(|r| r.as_array()) {
            for rt in runtimes_arr {
                let name = rt.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let version = rt.get("version").and_then(|v| v.as_str()).unwrap_or("");
                xml.push_str(&format!("<runtime name=\"{}\" version=\"{}\"/>", name, version));
            }
        }
        xml.push_str("</runtimes>");

        xml.push_str("<configuration>");
        xml.push_str(&serde_json::to_string(&context.config).unwrap_or_default());
        xml.push_str("</configuration>");

        xml.push_str("<diagnostics>");
        xml.push_str(&serde_json::to_string(&context.diagnostics).unwrap_or_default());
        xml.push_str("</diagnostics>");

        xml.push_str("<workspace_files>");
        if let Some(files) = context.workspace.as_array() {
            for f in files {
                let path = f.get("path").and_then(|p| p.as_str()).unwrap_or("");
                xml.push_str(&format!("<file path=\"{}\"/>", path));
            }
        }
        xml.push_str("</workspace_files>");

        xml.push_str("</forge_context>");
        Ok(xml)
    }
}

pub struct GeminiCliAdapter;

impl AgentAdapter for GeminiCliAdapter {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn adapt(&self, context: &ForgeContext, _exporter: &dyn ContextExporter) -> Result<String, String> {
        let payload = serde_json::json!({
            "systemInstructionContext": {
                "schema_version": context.schema_version,
                "runtimes": context.runtimes,
                "config": context.config,
                "diagnostics": context.diagnostics,
                "workspace": context.workspace,
                "environment": context.environment,
                "secrets_metadata": context.secrets_metadata,
                "toolAvailability": {
                    "fcp": true,
                    "diagnostics": true,
                    "runtimes": true
                }
            }
        });
        serde_json::to_string(&payload)
            .map_err(|e| format!("Failed to serialize Gemini context: {}", e))
    }
}

pub struct AiderAdapter;

impl AgentAdapter for AiderAdapter {
    fn name(&self) -> &'static str {
        "aider"
    }

    fn adapt(&self, context: &ForgeContext, _exporter: &dyn ContextExporter) -> Result<String, String> {
        let mut map_lines = Vec::new();
        map_lines.push("Aider Repository Map:".to_string());

        if let Some(files) = context.workspace.as_array() {
            for f in files {
                let path_str = f.get("path").and_then(|p| p.as_str()).unwrap_or("");
                if path_str.ends_with("README.md") {
                    continue;
                }

                if path_str.ends_with(".rs") {
                    map_lines.push(format!("File: {}", path_str));
                    // Simple parse if files are locally accessible
                    if let Ok(content) = std::fs::read_to_string(path_str) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("pub fn") || trimmed.starts_with("fn ") ||
                               trimmed.starts_with("pub struct") || trimmed.starts_with("struct ") ||
                               trimmed.starts_with("pub enum") || trimmed.starts_with("enum ") ||
                               trimmed.starts_with("pub trait") || trimmed.starts_with("trait ") ||
                               trimmed.starts_with("impl ") {
                                let clean = trimmed.trim_end_matches('{').trim();
                                map_lines.push(format!("  {}", clean));
                            }
                        }
                    }
                }
            }
        }

        Ok(map_lines.join("\n"))
    }
}

pub struct ContinueAdapter;

impl AgentAdapter for ContinueAdapter {
    fn name(&self) -> &'static str {
        "continue"
    }

    fn adapt(&self, context: &ForgeContext, exporter: &dyn ContextExporter) -> Result<String, String> {
        let md = exporter.export(context)?;
        let items = serde_json::json!([
            {
                "name": "Forge Context",
                "description": "Active Forge environment runtimes and workspace information",
                "content": md
            }
        ]);
        serde_json::to_string(&items)
            .map_err(|e| format!("Failed to serialize Continue adapter JSON: {}", e))
    }
}

// --- Unit Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct DummyProvider {
        name: &'static str,
        result: serde_json::Value,
    }

    impl ContextProvider for DummyProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        fn collect(&self, _options: &ContextOptions) -> Result<serde_json::Value, String> {
            Ok(self.result.clone())
        }
    }

    struct TimeoutProvider {
        sleep_ms: u64,
    }

    impl ContextProvider for TimeoutProvider {
        fn name(&self) -> &'static str {
            "diagnostics"
        }
        fn collect(&self, _options: &ContextOptions) -> Result<serde_json::Value, String> {
            std::thread::sleep(std::time::Duration::from_millis(self.sleep_ms));
            Ok(serde_json::json!({ "status": "ok" }))
        }
    }

    #[test]
    fn test_handshake_version_match() {
        let engine = ContextEngine::new();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "fcp.handshake",
            "params": {
                "version": "1.0.0",
                "capabilities": {
                    "scopes": ["workspace", "secrets", "runtimes"],
                    "exporters": ["json", "markdown"]
                }
            },
            "id": 42
        });

        let req_str = serde_json::to_string(&request).unwrap();
        let res_str = engine.handshake(&req_str).unwrap();
        let response: FcpHandshakeResponse = serde_json::from_str(&res_str).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id.as_i64().unwrap(), 42);
        assert_eq!(response.result.version, "1.0.0");
        assert!(response.result.negotiated_capabilities.scopes.contains(&"workspace".to_string()));
        assert!(response.result.negotiated_capabilities.scopes.contains(&"secrets".to_string()));
        assert!(response.result.negotiated_capabilities.scopes.contains(&"runtime".to_string())); // runtimes -> runtime
    }

    #[tokio::test]
    async fn test_provider_concurrency_with_timeouts() {
        let mut engine = ContextEngine::new();
        engine.register(Arc::new(DummyProvider {
            name: "runtime",
            result: serde_json::json!({ "active": "node" }),
        }));
        engine.register(Arc::new(TimeoutProvider { sleep_ms: 6000 })); // Will timeout (5000ms limit)

        let options = ContextOptions {
            workspace_root: std::env::current_dir().unwrap(),
            ..Default::default()
        };

        let context = engine.query(&options).await.unwrap();
        assert_eq!(context.runtimes, serde_json::json!({ "active": "node" }));
        assert_eq!(
            context.diagnostics.get("error").unwrap().as_str().unwrap(),
            "Provider query timed out after 5000ms"
        );
    }

    #[test]
    fn test_secret_masking() {
        let env_provider = EnvironmentProviderImpl;
        std::env::set_var("FORGE_TEST_DATABASE_PASSWORD", "supersecret123");
        std::env::set_var("FORGE_TEST_NORMAL_VAR", "hello-world");

        let options = ContextOptions::default();
        let result = env_provider.collect(&options).unwrap();
        assert_eq!(result.get("FORGE_TEST_DATABASE_PASSWORD").unwrap().as_str().unwrap(), "[MASKED]");
        assert_eq!(result.get("FORGE_TEST_NORMAL_VAR").unwrap().as_str().unwrap(), "hello-world");

        std::env::remove_var("FORGE_TEST_DATABASE_PASSWORD");
        std::env::remove_var("FORGE_TEST_NORMAL_VAR");
    }

    #[test]
    fn test_register_plugin_exporter_duplicate_rejection() {
        struct FirstExporter;
        impl ContextExporter for FirstExporter {
            fn name(&self) -> &'static str { "my-exporter" }
            fn export(&self, _context: &ForgeContext) -> Result<String, String> {
                Ok("first".to_string())
            }
        }

        struct SecondExporter;
        impl ContextExporter for SecondExporter {
            fn name(&self) -> &'static str { "my-exporter" }
            fn export(&self, _context: &ForgeContext) -> Result<String, String> {
                Ok("second".to_string())
            }
        }

        let mut engine = ContextEngine::new();
        // First registration should succeed
        assert!(engine.register_plugin_exporter(Arc::new(FirstExporter)).is_ok());
        // Second registration with same name should fail
        let err = engine.register_plugin_exporter(Arc::new(SecondExporter)).unwrap_err();
        assert!(err.contains("my-exporter"), "Error should mention the duplicate exporter name: {}", err);
    }

    #[test]
    fn test_workspace_limit_bounds() {
        let temp_dir = std::env::temp_dir().join("forge_workspace_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create structure to test depth 5 limit and 1000 file limits
        // Root is depth 0.
        // depth 1: d1
        // depth 2: d1/d2
        // depth 3: d1/d2/d3
        // depth 4: d1/d2/d3/d4
        // depth 5: d1/d2/d3/d4/d5
        // depth 6: d1/d2/d3/d4/d5/d6
        let mut current = temp_dir.clone();
        for i in 1..=6 {
            current = current.join(format!("d{}", i));
            fs::create_dir_all(&current).unwrap();
            fs::write(current.join(format!("file_{}.txt", i)), "hello").unwrap();
        }

        let provider = WorkspaceProviderImpl;
        let options = ContextOptions {
            workspace_root: temp_dir.clone(),
            ..Default::default()
        };

        let result = provider.collect(&options).unwrap();
        let files = result.as_array().unwrap();

        // Files at depth <= 5 should be found: file_1.txt (d1), file_2.txt (d1/d2), file_3.txt (d1/d2/d3), file_4.txt (d1/d2/d3/d4), file_5.txt (d1/d2/d3/d4/d5)
        // File at depth 6: d1/d2/d3/d4/d5/d6/file_6.txt should NOT be found
        let mut paths: Vec<String> = files.iter().map(|f| f.get("path").unwrap().as_str().unwrap().to_string()).collect();
        paths.sort();

        assert!(paths.iter().any(|p| p.ends_with("file_1.txt")));
        assert!(paths.iter().any(|p| p.ends_with("file_5.txt")));
        assert!(!paths.iter().any(|p| p.ends_with("file_6.txt")));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
