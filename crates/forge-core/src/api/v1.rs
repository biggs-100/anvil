use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::types::{Event, OperationStatus};
use crate::operations::{Context, SyncOperation, RepairOperation, CleanOperation, Operation};
use crate::secrets::{SecretProvider, ConfigurationProvider};
use crate::plugin::{Plugin, PluginError, PluginRegistry};
use crate::resolver::RuntimeProvider;
use crate::context::{ContextProvider, ContextExporter};
use crate::diagnostics::HealthCheck;
use std::io::{BufRead, Seek, SeekFrom};

pub type RuntimeDetail = RuntimeExplanation;
pub type TraceTree = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeExplanation {
    pub runtime: String,
    pub state: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSummary {
    pub id: String,
    pub runtime: String,
    pub duration_ms: u64,
    pub status: String,
}

#[derive(Debug)]
struct TreeNode {
    title: String,
    duration_ms: u64,
    children: Vec<TreeNode>,
}

pub struct Engine {
    pub workspace_root: PathBuf,
    pub cache_dir: PathBuf,
    pub plugin_registry: PluginRegistry,

    // ── Plugin extensions (drained from registry after init) ──
    /// Runtime providers contributed by plugins.
    pub plugin_runtime_providers: Vec<Box<dyn RuntimeProvider>>,
    /// Configuration providers contributed by plugins.
    pub plugin_config_providers: Vec<Box<dyn ConfigurationProvider>>,
    /// Custom operations contributed by plugins.
    pub plugin_operations: Vec<Box<dyn Operation>>,
    /// Context providers contributed by plugins (Arc-shared).
    pub plugin_context_providers: Vec<Arc<dyn ContextProvider>>,
    /// Context exporters contributed by plugins (Arc-shared).
    pub plugin_context_exporters: Vec<Arc<dyn ContextExporter>>,
    /// Health checks contributed by plugins (Arc-shared).
    pub plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
}

impl Engine {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let cache_dir = crate::cache::get_cache_dir()?;
        Ok(Self {
            workspace_root: root,
            cache_dir,
            plugin_registry: PluginRegistry::new(),
            plugin_runtime_providers: Vec::new(),
            plugin_config_providers: Vec::new(),
            plugin_operations: Vec::new(),
            plugin_context_providers: Vec::new(),
            plugin_context_exporters: Vec::new(),
            plugin_health_checks: Vec::new(),
        })
    }

    /// Registers a plugin with the engine's registry.
    ///
    /// The plugin is stored pending initialization. Call
    /// `resolve_and_init()` on the engine's registry to resolve
    /// dependencies and initialise all registered plugins.
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        self.plugin_registry.register(plugin)
    }

    /// Creates an Engine, registers the given plugins, and
    /// runs dependency resolution and initialisation.
    ///
    /// Returns an error if any plugin fails registration or init.
    pub fn new_with_plugins(
        root: PathBuf,
        plugins: Vec<Box<dyn Plugin>>,
    ) -> Result<Self, String> {
        let mut engine = Self::new(root)?;
        for plugin in plugins {
            engine
                .plugin_registry
                .register(plugin)
                .map_err(|e| format!("Plugin registration failed: {}", e))?;
        }
        engine
            .plugin_registry
            .resolve_and_init()
            .map_err(|e| format!("Plugin init failed: {}", e))?;

        // Drain plugin extensions from the registry into engine fields
        // so they are available to all engine methods.
        engine.plugin_runtime_providers = engine.plugin_registry.drain_runtime_providers();
        engine.plugin_config_providers = engine.plugin_registry.drain_config_providers();
        engine.plugin_operations = engine.plugin_registry.drain_operations();
        // Arc-based extensions: clone from registry (shared ownership)
        engine.plugin_context_providers = engine.plugin_registry.context_providers();
        engine.plugin_context_exporters = engine.plugin_registry.context_exporters();
        engine.plugin_health_checks = engine.plugin_registry.health_checks();

        Ok(engine)
    }

    /// Returns the plugin runtime providers registered via plugins.
    pub fn runtime_providers(&self) -> &[Box<dyn RuntimeProvider>] {
        &self.plugin_runtime_providers
    }

    /// Returns the plugin configuration providers registered via plugins.
    pub fn config_providers(&self) -> &[Box<dyn ConfigurationProvider>] {
        &self.plugin_config_providers
    }

    /// Returns the plugin operations registered via plugins.
    pub fn operations(&self) -> &[Box<dyn Operation>] {
        &self.plugin_operations
    }

    /// Returns the plugin context providers registered via plugins.
    pub fn context_providers(&self) -> &[Arc<dyn ContextProvider>] {
        &self.plugin_context_providers
    }

    /// Returns the plugin context exporters registered via plugins.
    pub fn context_exporters(&self) -> &[Arc<dyn ContextExporter>] {
        &self.plugin_context_exporters
    }

    /// Returns the plugin health checks registered via plugins.
    pub fn health_checks(&self) -> &[Arc<dyn HealthCheck>] {
        &self.plugin_health_checks
    }

    /// Finds a plugin operation by name.
    pub fn find_plugin_operation(&self, name: &str) -> Option<&dyn Operation> {
        self.plugin_operations
            .iter()
            .find(|op| op.name() == name)
            .map(|op| op.as_ref())
    }

    pub async fn get_status(&self) -> Result<String, String> {
        let state = crate::state::compute_current_state(&self.workspace_root, &self.cache_dir);
        Ok(format!("{:?}", state))
    }

    pub async fn sync(&self) -> Result<(), String> {
        let event_bus = crate::event_bus::EventBus::new(100);
        let mut ctx = Context::new(self.workspace_root.clone(), self.cache_dir.clone(), event_bus);
        let _ = ctx.load_config();
        let _ = ctx.load_lockfile();
        
        let op = SyncOperation;
        let plan = op.plan(&ctx)?;
        let result = op.execute(&mut ctx, plan).await?;
        
        // Compute and save state
        let final_state = crate::state::compute_current_state(&self.workspace_root, &self.cache_dir);
        crate::state::save_state(&self.workspace_root, final_state);
        
        if result.status == OperationStatus::Failure {
            return Err("Sync operation failed".to_string());
        }
        Ok(())
    }

    pub async fn repair(&self) -> Result<(), String> {
        let event_bus = crate::event_bus::EventBus::new(100);
        let mut ctx = Context::new(self.workspace_root.clone(), self.cache_dir.clone(), event_bus);
        let _ = ctx.load_config();
        let _ = ctx.load_lockfile();
        
        let op = RepairOperation;
        let plan = op.plan(&ctx)?;
        let result = op.execute(&mut ctx, plan).await?;
        
        // Compute and save state
        let final_state = crate::state::compute_current_state(&self.workspace_root, &self.cache_dir);
        crate::state::save_state(&self.workspace_root, final_state);
        
        if result.status == OperationStatus::Failure {
            return Err("Repair operation failed".to_string());
        }
        Ok(())
    }

    pub async fn clean(&self) -> Result<(), String> {
        let event_bus = crate::event_bus::EventBus::new(100);
        let mut ctx = Context::new(self.workspace_root.clone(), self.cache_dir.clone(), event_bus);
        let _ = ctx.load_config();
        let _ = ctx.load_lockfile();
        
        let op = CleanOperation;
        let plan = op.plan(&ctx)?;
        let result = op.execute(&mut ctx, plan).await?;
        
        // Compute and save state
        let final_state = crate::state::compute_current_state(&self.workspace_root, &self.cache_dir);
        crate::state::save_state(&self.workspace_root, final_state);
        
        if result.status == OperationStatus::Failure {
            return Err("Clean operation failed".to_string());
        }
        Ok(())
    }

    pub async fn explain(&self, runtime: &str) -> Result<RuntimeExplanation, String> {
        let mut diagnostics = Vec::new();
        let toml_path = self.workspace_root.join("forge.toml");
        if !toml_path.exists() {
            return Err("No forge.toml manifest found".to_string());
        }
        let config = crate::manifest::load_config(&toml_path)?;
        let version_req = match config.runtimes.get(runtime) {
            Some(v) => v,
            None => return Err(format!("Runtime '{}' is not configured in forge.toml", runtime)),
        };

        diagnostics.push(format!("Configured version requirement: {}", version_req));

        let lock_path = self.workspace_root.join("forge.lock");
        if !lock_path.exists() {
            return Ok(RuntimeExplanation {
                runtime: runtime.to_string(),
                state: "Initialized".to_string(),
                diagnostics: vec!["forge.lock is missing".to_string()],
            });
        }
        let lockfile = crate::load_lockfile(&lock_path)?;
        let lock_entry = lockfile.runtimes.iter().find(|r| r.name == runtime);

        let lock_entry = match lock_entry {
            Some(e) => e,
            None => {
                diagnostics.push("Not present in forge.lock".to_string());
                return Ok(RuntimeExplanation {
                    runtime: runtime.to_string(),
                    state: "Outdated".to_string(),
                    diagnostics,
                });
            }
        };

        diagnostics.push(format!("Locked version: {}", lock_entry.version));
        diagnostics.push(format!("Download URL: {}", lock_entry.url));
        diagnostics.push(format!("SHA-256: {}", lock_entry.sha256));

        let extract_dir = self.cache_dir.join(runtime).join(&lock_entry.version).join("extracted");
        let is_extracted = extract_dir.exists() && {
            if let Ok(mut entries) = std::fs::read_dir(&extract_dir) {
                entries.next().is_some()
            } else {
                false
            }
        };

        let state = if is_extracted {
            diagnostics.push(format!("Cache path: {}", extract_dir.display()));
            let shims_cache = self.workspace_root.join(".forge").join("shims.cache");
            if shims_cache.exists() {
                diagnostics.push(format!("Shims cache: Present ({})", shims_cache.display()));
                "Ready".to_string()
            } else {
                diagnostics.push("Shims cache: Missing".to_string());
                "Synced".to_string()
            }
        } else {
            diagnostics.push("Cache status: Missing/Not extracted".to_string());
            "Locked".to_string()
        };

        Ok(RuntimeExplanation {
            runtime: runtime.to_string(),
            state,
            diagnostics,
        })
    }

    pub async fn read_history(&self) -> Result<Vec<OperationSummary>, String> {
        let journal_path = self.workspace_root.join(".forge").join("journal.jsonl");
        if !journal_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&journal_path)
            .map_err(|e| format!("Failed to open journal file: {}", e))?;
        
        use std::io::BufReader;
        let reader = BufReader::new(file);
        
        let mut op_events: HashMap<String, Vec<Event>> = HashMap::new();
        let mut op_order: Vec<String> = Vec::new();

        for line in reader.lines().flatten() {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                let op_id = event.operation_id.clone();
                if !op_events.contains_key(&op_id) {
                    op_order.push(op_id.clone());
                }
                op_events.entry(op_id).or_default().push(event);
            }
        }

        let mut summaries = Vec::new();
        for op_id in op_order {
            if let Some(events) = op_events.get(&op_id) {
                if events.is_empty() {
                    continue;
                }
                
                // Get runtime (first non-all, or all)
                let runtime = events.iter()
                    .map(|e| e.runtime.clone())
                    .find(|r| r != "all")
                    .unwrap_or_else(|| "all".to_string());

                // Find status
                let status = if events.iter().any(|e| matches!(e.status, crate::types::EventStatus::Failed(_))) {
                    "Failure".to_string()
                } else if events.iter().any(|e| matches!(e.status, crate::types::EventStatus::Finished)) {
                    "Success".to_string()
                } else {
                    "Running".to_string()
                };

                // Calculate duration
                let mut min_ms = u64::MAX;
                let mut max_ms = 0;
                for event in events {
                    if let Some(ms) = parse_timestamp_to_ms(&event.timestamp) {
                        if ms < min_ms {
                            min_ms = ms;
                        }
                        if ms > max_ms {
                            max_ms = ms;
                        }
                    }
                }
                let duration_ms = if min_ms <= max_ms { max_ms - min_ms } else { 0 };

                summaries.push(OperationSummary {
                    id: op_id,
                    runtime,
                    duration_ms,
                    status,
                });
            }
        }

        // Return sorted by chronological reverse order of their first event
        Ok(summaries.into_iter().rev().collect())
    }

    pub async fn history(&self, limit: Option<usize>) -> Result<Vec<OperationSummary>, String> {
        let mut history = self.read_history().await?;
        if let Some(n) = limit {
            history.truncate(n);
        }
        Ok(history)
    }

    pub async fn trace_operation(&self, id: &str) -> Result<String, String> {
        self.trace(id).await
    }

    pub async fn trace(&self, id: &str) -> Result<TraceTree, String> {
        let journal_path = self.workspace_root.join(".forge").join("journal.jsonl");
        if !journal_path.exists() {
            return Err("Journal file not found".to_string());
        }

        let file = std::fs::File::open(&journal_path)
            .map_err(|e| format!("Failed to open journal file: {}", e))?;
        
        use std::io::BufReader;
        let reader = BufReader::new(file);
        
        let mut events = Vec::new();
        for line in reader.lines().flatten() {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                if event.operation_id == id {
                    events.push(event);
                }
            }
        }

        if events.is_empty() {
            return Err(format!("No events found for operation '{}'", id));
        }

        // Build hierarchy tree
        // Node: phase or runtime block
        #[derive(Debug)]
        struct RawNode {
            runtime: String,
            phase: String,
            start_ms: u64,
            end_ms: u64,
        }

        // Extract phases and durations
        let mut phases: HashMap<(String, String), (u64, u64)> = HashMap::new();
        let mut phase_order = Vec::new();

        for event in &events {
            let key = (event.runtime.clone(), event.phase.clone());
            if let Some(ms) = parse_timestamp_to_ms(&event.timestamp) {
                let entry = phases.entry(key.clone()).or_insert((ms, ms));
                if ms < entry.0 {
                    entry.0 = ms;
                }
                if ms > entry.1 {
                    entry.1 = ms;
                }
                if !phase_order.contains(&key) {
                    phase_order.push(key);
                }
            }
        }

        let mut raw_nodes = Vec::new();
        for key in phase_order {
            if let Some(&(start, end)) = phases.get(&key) {
                raw_nodes.push(RawNode {
                    runtime: key.0,
                    phase: key.1,
                    start_ms: start,
                    end_ms: end,
                });
            }
        }

        let mut root = TreeNode {
            title: "".to_string(),
            duration_ms: 0,
            children: Vec::new(),
        };

        // Group raw nodes. Top-level are where runtime == "all"
        // If runtime != "all", group them under "Runtime: <runtime>" parent node
        let mut runtime_group_indices: HashMap<String, usize> = HashMap::new();

        for node in raw_nodes {
            let duration = if node.end_ms >= node.start_ms { node.end_ms - node.start_ms } else { 0 };
            if node.runtime == "all" {
                root.children.push(TreeNode {
                    title: node.phase,
                    duration_ms: duration,
                    children: Vec::new(),
                });
            } else {
                let parent_name = format!("Runtime: {}", node.runtime);
                let idx = if let Some(&idx) = runtime_group_indices.get(&parent_name) {
                    idx
                } else {
                    let new_idx = root.children.len();
                    root.children.push(TreeNode {
                        title: parent_name.clone(),
                        duration_ms: 0, // will accumulate
                        children: Vec::new(),
                    });
                    runtime_group_indices.insert(parent_name.clone(), new_idx);
                    new_idx
                };
                
                // Add phase child
                root.children[idx].children.push(TreeNode {
                    title: node.phase,
                    duration_ms: duration,
                    children: Vec::new(),
                });
                root.children[idx].duration_ms += duration;
            }
        }

        let mut out = String::new();
        out.push_str(&format!("Operation: {}\n", id));
        let child_count = root.children.len();
        for (i, child) in root.children.iter().enumerate() {
            print_tree(child, "", i == child_count - 1, &mut out);
        }

        Ok(out)
    }

    pub async fn events(&self, live: bool) -> Result<tokio::sync::mpsc::Receiver<Event>, String> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let journal_path = self.workspace_root.join(".forge").join("journal.jsonl");
        
        if !live {
            if journal_path.exists() {
                if let Ok(file) = std::fs::File::open(&journal_path) {
                    use std::io::BufReader;
                    let reader = BufReader::new(file);
                    for line in reader.lines().flatten() {
                        if let Ok(event) = serde_json::from_str::<Event>(&line) {
                            let _ = tx.send(event).await;
                        }
                    }
                }
            }
        } else {
            tokio::spawn(async move {
                let mut last_position = 0;
                if journal_path.exists() {
                    if let Ok(file) = std::fs::File::open(&journal_path) {
                        use std::io::BufReader;
                        let mut reader = BufReader::new(file);
                        let mut line = String::new();
                        while reader.read_line(&mut line).unwrap_or(0) > 0 {
                            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                                let _ = tx.send(event).await;
                            }
                            line.clear();
                        }
                        if let Ok(pos) = reader.stream_position() {
                            last_position = pos;
                        }
                    }
                }
                
                loop {
                    if journal_path.exists() {
                        if let Ok(metadata) = std::fs::metadata(&journal_path) {
                            let len = metadata.len();
                            if len > last_position {
                                if let Ok(file) = std::fs::File::open(&journal_path) {
                                    use std::io::BufReader;
                                    let mut reader = BufReader::new(file);
                                    if reader.seek(SeekFrom::Start(last_position)).is_ok() {
                                        let mut line = String::new();
                                        while reader.read_line(&mut line).unwrap_or(0) > 0 {
                                            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                                                if tx.send(event).await.is_err() {
                                                    return;
                                                }
                                            }
                                            line.clear();
                                        }
                                    }
                                }
                                last_position = len;
                            }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
        }
        
        Ok(rx)
    }

    pub async fn env_list(&self) -> Result<HashMap<String, String>, String> {
        let env_path = crate::environment::find_forge_env(&self.workspace_root);
        if let Some(path) = env_path {
            crate::environment::parse_env_file(&path)
        } else {
            Ok(HashMap::new())
        }
    }

    pub async fn env_get(&self, key: &str) -> Result<Option<String>, String> {
        let list = self.env_list().await?;
        Ok(list.get(key).cloned())
    }

    pub async fn env_set(&self, key: &str, value: &str) -> Result<(), String> {
        let env_path = crate::environment::find_forge_env(&self.workspace_root)
            .unwrap_or_else(|| self.workspace_root.join("forge.env"));
        
        let mut list = if env_path.exists() {
            crate::environment::parse_env_file(&env_path)?
        } else {
            HashMap::new()
        };

        list.insert(key.to_string(), value.to_string());

        let mut lines = Vec::new();
        for (k, v) in &list {
            lines.push(format!("{}={}", k, v));
        }
        std::fs::write(&env_path, lines.join("\n"))
            .map_err(|e| format!("Failed to write forge.env: {}", e))?;
        Ok(())
    }

    pub async fn env_unset(&self, key: &str) -> Result<(), String> {
        let env_path = crate::environment::find_forge_env(&self.workspace_root);
        if let Some(path) = env_path {
            let mut list = crate::environment::parse_env_file(&path)?;
            if list.remove(key).is_some() {
                let mut lines = Vec::new();
                for (k, v) in &list {
                    lines.push(format!("{}={}", k, v));
                }
                std::fs::write(&path, lines.join("\n"))
                    .map_err(|e| format!("Failed to write forge.env: {}", e))?;
            }
        }
        Ok(())
    }

    pub async fn env_resolve(&self, profile: Option<&str>) -> Result<crate::secrets::ResolvedEnvironment, String> {
        struct EngineRuntimeContextProvider {
            workspace_root: PathBuf,
            cache_dir: PathBuf,
        }
        impl crate::environment::RuntimeContextProvider for EngineRuntimeContextProvider {
            fn workspace_root(&self) -> &Path {
                &self.workspace_root
            }
            fn runtime_path(&self, name: &str) -> Option<PathBuf> {
                let lock_path = self.workspace_root.join("forge.lock");
                if lock_path.exists() {
                    if let Ok(lockfile) = crate::load_lockfile(&lock_path) {
                        if let Some(runtime) = lockfile.runtimes.iter().find(|r| r.name == name) {
                            return Some(self.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted"));
                        }
                    }
                }
                None
            }
        }

        let ctx = EngineRuntimeContextProvider {
            workspace_root: self.workspace_root.clone(),
            cache_dir: self.cache_dir.clone(),
        };

        if self.plugin_config_providers.is_empty() {
            crate::environment::materialize_environment(&ctx, &HashMap::new(), profile)
        } else {
            crate::environment::materialize_environment_with_plugins(
                &ctx,
                &HashMap::new(),
                profile,
                &self.plugin_config_providers,
            )
        }
    }

    pub async fn secret_set(&self, key: &str, value: &str) -> Result<(), String> {
        use sha2::Digest;
        let toml_path = self.workspace_root.join("forge.toml");
        let config = if toml_path.exists() {
            crate::manifest::load_config(&toml_path).ok()
        } else {
            None
        };
        let workspace_id = config
            .as_ref()
            .and_then(|c| c.workspace_id.clone())
            .unwrap_or_else(|| {
                let hash_bytes = sha2::Sha256::digest(self.workspace_root.to_string_lossy().as_bytes());
                hex::encode(&hash_bytes[..8])
            });

        let secrets_manifest_path = self.workspace_root.join("forge.secrets");
        let mut provider_name = "file".to_string();

        #[derive(Serialize, Deserialize, Default)]
        struct SecretsManifest {
            #[serde(default)]
            secrets: HashMap<String, SecretConfig>,
        }
        #[derive(Serialize, Deserialize, Clone)]
        struct SecretConfig {
            provider: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            secret_id: Option<String>,
        }

        let mut manifest = if secrets_manifest_path.exists() {
            let content = std::fs::read_to_string(&secrets_manifest_path)
                .map_err(|e| format!("Failed to read forge.secrets: {}", e))?;
            toml::from_str::<SecretsManifest>(&content)
                .map_err(|e| format!("Failed to parse forge.secrets: {}", e))?
        } else {
            SecretsManifest::default()
        };

        if let Some(cfg) = manifest.secrets.get(key) {
            provider_name = cfg.provider.clone();
        } else {
            let test_entry = keyring::Entry::new("forge-secrets", &format!("{}::_test_conn", workspace_id));
            if test_entry.is_ok() && test_entry.unwrap().set_password("test").is_ok() {
                provider_name = "keyring".to_string();
            }
            manifest.secrets.insert(key.to_string(), SecretConfig {
                provider: provider_name.clone(),
                key: None,
                secret_id: None,
            });
            let toml_content = toml::to_string_pretty(&manifest)
                .map_err(|e| format!("Failed to serialize forge.secrets: {}", e))?;
            std::fs::write(&secrets_manifest_path, toml_content)
                .map_err(|e| format!("Failed to write forge.secrets: {}", e))?;
        }

        if provider_name == "keyring" {
            let provider = crate::secrets::KeyringSecretProvider::new(&workspace_id);
            provider.set(key, value)?;
        } else {
            let provider = crate::secrets::FallbackSecretProvider::new(
                &workspace_id,
                self.workspace_root.join(".forge").join("secrets.enc")
            );
            provider.set(key, value)?;
        }

        Ok(())
    }

    pub async fn secret_get(&self, key: &str) -> Result<Option<String>, String> {
        use sha2::Digest;
        let toml_path = self.workspace_root.join("forge.toml");
        let config = if toml_path.exists() {
            crate::manifest::load_config(&toml_path).ok()
        } else {
            None
        };
        let workspace_id = config
            .as_ref()
            .and_then(|c| c.workspace_id.clone())
            .unwrap_or_else(|| {
                let hash_bytes = sha2::Sha256::digest(self.workspace_root.to_string_lossy().as_bytes());
                hex::encode(&hash_bytes[..8])
            });

        let secrets_manifest_path = self.workspace_root.join("forge.secrets");
        if !secrets_manifest_path.exists() {
            return Ok(None);
        }

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

        let content = std::fs::read_to_string(&secrets_manifest_path)
            .map_err(|e| format!("Failed to read forge.secrets: {}", e))?;
        let manifest = toml::from_str::<SecretsManifest>(&content)
            .map_err(|e| format!("Failed to parse forge.secrets: {}", e))?;

        let cfg = match manifest.secrets.get(key) {
            Some(c) => c,
            None => return Ok(None),
        };

        let lookup_key = cfg.key.as_deref().or(cfg.secret_id.as_deref()).unwrap_or(key);

        if cfg.provider == "keyring" {
            let provider = crate::secrets::KeyringSecretProvider::new(&workspace_id);
            provider.get(lookup_key)
        } else {
            let provider = crate::secrets::FallbackSecretProvider::new(
                &workspace_id,
                self.workspace_root.join(".forge").join("secrets.enc")
            );
            provider.get(lookup_key)
        }
    }

    pub async fn secret_list(&self) -> Result<Vec<String>, String> {
        let secrets_manifest_path = self.workspace_root.join("forge.secrets");
        if !secrets_manifest_path.exists() {
            return Ok(Vec::new());
        }
        #[derive(Deserialize)]
        struct SecretsManifest {
            secrets: HashMap<String, serde_json::Value>,
        }
        let content = std::fs::read_to_string(&secrets_manifest_path)
            .map_err(|e| format!("Failed to read forge.secrets: {}", e))?;
        let manifest: SecretsManifest = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse forge.secrets: {}", e))?;
        Ok(manifest.secrets.keys().cloned().collect())
    }

    pub async fn secret_remove(&self, key: &str) -> Result<(), String> {
        use sha2::Digest;
        let toml_path = self.workspace_root.join("forge.toml");
        let config = if toml_path.exists() {
            crate::manifest::load_config(&toml_path).ok()
        } else {
            None
        };
        let workspace_id = config
            .as_ref()
            .and_then(|c| c.workspace_id.clone())
            .unwrap_or_else(|| {
                let hash_bytes = sha2::Sha256::digest(self.workspace_root.to_string_lossy().as_bytes());
                hex::encode(&hash_bytes[..8])
            });

        let secrets_manifest_path = self.workspace_root.join("forge.secrets");
        if !secrets_manifest_path.exists() {
            return Ok(());
        }

        #[derive(Serialize, Deserialize)]
        struct SecretsManifest {
            secrets: HashMap<String, SecretConfig>,
        }
        #[derive(Serialize, Deserialize)]
        struct SecretConfig {
            provider: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            secret_id: Option<String>,
        }

        let content = std::fs::read_to_string(&secrets_manifest_path)
            .map_err(|e| format!("Failed to read forge.secrets: {}", e))?;
        let mut manifest = toml::from_str::<SecretsManifest>(&content)
            .map_err(|e| format!("Failed to parse forge.secrets: {}", e))?;

        if let Some(cfg) = manifest.secrets.remove(key) {
            let lookup_key = cfg.key.as_deref().or(cfg.secret_id.as_deref()).unwrap_or(key);
            if cfg.provider == "keyring" {
                let provider = crate::secrets::KeyringSecretProvider::new(&workspace_id);
                let _ = provider.delete(lookup_key);
            } else {
                let provider = crate::secrets::FallbackSecretProvider::new(
                    &workspace_id,
                    self.workspace_root.join(".forge").join("secrets.enc")
                );
                let _ = provider.delete(lookup_key);
            }
            let toml_content = toml::to_string_pretty(&manifest)
                .map_err(|e| format!("Failed to serialize forge.secrets: {}", e))?;
            std::fs::write(&secrets_manifest_path, toml_content)
                .map_err(|e| format!("Failed to write forge.secrets: {}", e))?;
        }

        Ok(())
    }

    pub async fn secret_export(&self) -> Result<HashMap<String, String>, String> {
        let mut exported = HashMap::new();
        let keys = self.secret_list().await?;
        for key in keys {
            if let Some(val) = self.secret_get(&key).await? {
                exported.insert(key, val);
            }
        }
        Ok(exported)
    }

    pub async fn secret_import(&self, secrets: &HashMap<String, String>) -> Result<(), String> {
        for (key, val) in secrets {
            self.secret_set(key, val).await?;
        }
        Ok(())
    }

    pub async fn secret_doctor(&self) -> Result<Vec<String>, String> {
        use sha2::Digest;
        let mut report = Vec::new();
        let toml_path = self.workspace_root.join("forge.toml");
        let config = if toml_path.exists() {
            crate::manifest::load_config(&toml_path).ok()
        } else {
            None
        };
        let workspace_id = config
            .as_ref()
            .and_then(|c| c.workspace_id.clone())
            .unwrap_or_else(|| {
                let hash_bytes = sha2::Sha256::digest(self.workspace_root.to_string_lossy().as_bytes());
                hex::encode(&hash_bytes[..8])
            });

        let test_entry = keyring::Entry::new("forge-secrets", &format!("{}::_test_conn", workspace_id));
        match test_entry {
            Ok(entry) => {
                match entry.set_password("test_payload") {
                    Ok(_) => {
                        let _ = entry.delete_password();
                        report.push("OS Keyring: Connected and healthy".to_string());
                    }
                    Err(e) => {
                        report.push(format!("OS Keyring: Write failed (using fallback). Error: {}", e));
                    }
                }
            }
            Err(e) => {
                report.push(format!("OS Keyring: Connection failed (using fallback). Error: {}", e));
            }
        }

        let enc_file = self.workspace_root.join(".forge").join("secrets.enc");
        if enc_file.exists() {
            report.push(format!("Fallback encrypted store: Present ({})", enc_file.display()));
            if let Ok(_phrase) = crate::secrets::get_passphrase() {
                let provider = crate::secrets::FallbackSecretProvider::new(&workspace_id, enc_file.clone());
                match provider.list() {
                    Ok(keys) => {
                        let keys: Vec<String> = keys;
                        report.push(format!("Fallback store decryption: Successful ({} secrets found)", keys.len()));
                    }
                    Err(e) => {
                        report.push(format!("Fallback store decryption: Failed. Error: {}", e));
                    }
                }
            } else {
                report.push("Fallback store decryption: Passphrase not configured (FORGE_MASTER_KEY not set)".to_string());
            }
        } else {
            report.push("Fallback encrypted store: Not found (no local encrypted secrets)".to_string());
        }

        Ok(report)
    }
}

fn print_tree(node: &TreeNode, prefix: &str, is_last: bool, out: &mut String) {
    let marker = if is_last { "└── " } else { "├── " };
    out.push_str(&format!("{}{}{} ({}ms)\n", prefix, marker, node.title, node.duration_ms));
    let child_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let new_prefix = if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };
        print_tree(child, &new_prefix, i == child_count - 1, out);
    }
}

fn parse_timestamp_to_ms(ts: &str) -> Option<u64> {
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() < 2 { return None; }
    let date_part = parts[0];
    let time_zone_part = parts[1];
    
    let ymd: Vec<&str> = date_part.split('-').collect();
    if ymd.len() < 3 { return None; }
    let year = ymd[0].parse::<u32>().ok()?;
    let month = ymd[1].parse::<u32>().ok()?;
    let day = ymd[2].parse::<u32>().ok()?;
    
    let tz_idx = time_zone_part.find(|c| c == 'Z' || c == '+' || c == '-').unwrap_or(time_zone_part.len());
    let time_str = &time_zone_part[..tz_idx];
    let tz_str = &time_zone_part[tz_idx..];
    
    let hms_parts: Vec<&str> = time_str.split(':').collect();
    if hms_parts.len() < 3 { return None; }
    let hour = hms_parts[0].parse::<u32>().ok()?;
    let minute = hms_parts[1].parse::<u32>().ok()?;
    
    let sec_parts: Vec<&str> = hms_parts[2].split('.').collect();
    let second = sec_parts[0].parse::<u32>().ok()?;
    let ms = if sec_parts.len() > 1 {
        let ms_str = sec_parts[1];
        let ms_str = if ms_str.len() > 3 { &ms_str[..3] } else { ms_str };
        let mut ms_val = ms_str.parse::<u32>().ok().unwrap_or(0);
        if ms_str.len() == 1 { ms_val *= 100; }
        else if ms_str.len() == 2 { ms_val *= 10; }
        ms_val
    } else {
        0
    };
    
    let days_in_months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut total_days = 0;
    for y in 1970..year {
        let is_leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
        total_days += if is_leap { 366 } else { 365 };
    }
    for m in 1..month {
        total_days += days_in_months[m as usize];
        if m == 2 {
            let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            if is_leap { total_days += 1; }
        }
    }
    total_days += day - 1;
    
    let mut total_seconds = total_days as u64 * 86400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64;
    
    if tz_str.starts_with('+') || tz_str.starts_with('-') {
        let sign = if tz_str.starts_with('+') { -1 } else { 1 };
        let tz_hm: Vec<&str> = tz_str[1..].split(':').collect();
        if tz_hm.len() >= 2 {
            let tz_h = tz_hm[0].parse::<i64>().unwrap_or(0);
            let tz_m = tz_hm[1].parse::<i64>().unwrap_or(0);
            let offset_sec = tz_h * 3600 + tz_m * 60;
            if sign == 1 {
                total_seconds = total_seconds.saturating_add(offset_sec as u64);
            } else {
                total_seconds = total_seconds.saturating_sub(offset_sec as u64);
            }
        }
    }
    
    Some(total_seconds * 1000 + ms as u64)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CliCommand, ExtensionSink, Plugin};

    /// 4.4: Integration — Engine::register_plugin() → resolve → init → query extension types.
    #[test]
    fn test_engine_register_plugin() {
        use crate::resolver::RuntimeProvider;
        use crate::registry::HybridRegistry;
        use crate::types::RuntimeLock;

        struct TestRuntimePlugin;

        impl Plugin for TestRuntimePlugin {
            fn name(&self) -> &str {
                "engine-test-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String> {
                sink.add_runtime_provider(Box::new(TestRuntimeProvider));
                sink.add_cli_command(Box::new(TestCliCommand));
                Ok(())
            }
        }

        struct TestRuntimeProvider;

        impl RuntimeProvider for TestRuntimeProvider {
            fn name(&self) -> &str {
                "engine-test-runtime"
            }
            fn resolve(
                &self,
                _version_req: &str,
                _platform: &str,
                _arch: &str,
                _registry: &HybridRegistry,
            ) -> Result<RuntimeLock, String> {
                Err("engine-test-runtime resolve called".to_string())
            }
        }

        struct TestCliCommand;

        impl CliCommand for TestCliCommand {
            fn name(&self) -> &str {
                "engine-test-cmd"
            }
            fn description(&self) -> &str {
                "Engine test command"
            }
            fn execute(&self, _args: &[String]) -> Result<(), String> {
                Ok(())
            }
        }

        let temp_dir = std::env::temp_dir().join("forge_plugin_engine_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut engine = Engine::new(temp_dir.clone()).unwrap();

        // Register plugin before init
        engine
            .register_plugin(Box::new(TestRuntimePlugin))
            .unwrap();

        // Resolve and init
        engine.plugin_registry.resolve_and_init().unwrap();

        // Query extension types
        let providers = engine.plugin_registry.runtime_providers();
        assert!(!providers.is_empty());
        assert_eq!(providers[0].name(), "engine-test-runtime");

        let cmds = engine.plugin_registry.cli_commands();
        assert!(!cmds.is_empty());
        assert_eq!(cmds[0].name(), "engine-test-cmd");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// Test new_with_plugins factory.
    #[test]
    fn test_new_with_plugins() {
        struct FactoryPlugin;

        impl Plugin for FactoryPlugin {
            fn name(&self) -> &str {
                "factory-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, _sink: &mut dyn ExtensionSink) -> Result<(), String> {
                Ok(())
            }
        }

        let temp_dir = std::env::temp_dir().join("forge_plugin_factory_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let engine =
            Engine::new_with_plugins(temp_dir.clone(), vec![Box::new(FactoryPlugin)]).unwrap();
        assert!(engine.plugin_registry.is_initialized());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
