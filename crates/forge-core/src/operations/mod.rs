use std::path::PathBuf;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use serde_json::json;
use crate::event_bus::EventBus;
use crate::types::{Lockfile, RuntimeLock, OperationResult, OperationStatus, ChangeRecord, Event, EventStatus};
use crate::manifest::{ForgeConfig, load_config};
use crate::lock::{load_lockfile};
use crate::cache::{regenerate_shims_cache, find_bin_dirs};

pub trait Plan: std::any::Any + Send + Sync {
    fn to_json(&self) -> serde_json::Value;
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Boxed future type alias for dyn-compatible Operation::execute.
pub type OperationFuture<'a> = Pin<Box<dyn Future<Output = Result<OperationResult, String>> + Send + 'a>>;

pub trait Operation: Send + Sync {
    fn name(&self) -> &str;
    fn plan(&self, ctx: &Context) -> Result<Box<dyn Plan>, String>;
    fn execute<'a>(&'a self, ctx: &'a mut Context, plan: Box<dyn Plan>) -> OperationFuture<'a>;
}

pub struct Context {
    pub workspace_root: PathBuf,
    pub cache_dir: PathBuf,
    pub event_bus: EventBus,
    pub operation_id: String,
    pub config: Option<ForgeConfig>,
    pub lockfile: Option<Lockfile>,
}

impl Context {
    pub fn new(workspace_root: PathBuf, cache_dir: PathBuf, event_bus: EventBus) -> Self {
        let operation_id = format!(
            "op-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        Self {
            workspace_root,
            cache_dir,
            event_bus,
            operation_id,
            config: None,
            lockfile: None,
        }
    }

    pub fn load_config(&mut self) -> Result<&ForgeConfig, String> {
        if self.config.is_none() {
            let toml_path = self.workspace_root.join("forge.toml");
            if !toml_path.exists() {
                return Err("No forge.toml manifest found".to_string());
            }
            let config = load_config(&toml_path)?;
            self.config = Some(config);
        }
        Ok(self.config.as_ref().unwrap())
    }

    pub fn load_lockfile(&mut self) -> Result<&Lockfile, String> {
        if self.lockfile.is_none() {
            let lock_path = self.workspace_root.join("forge.lock");
            let lockfile = if lock_path.exists() {
                load_lockfile(&lock_path)?
            } else {
                Lockfile::default()
            };
            self.lockfile = Some(lockfile);
        }
        Ok(self.lockfile.as_ref().unwrap())
    }
}

impl crate::environment::RuntimeContextProvider for Context {
    fn workspace_root(&self) -> &std::path::Path {
        &self.workspace_root
    }

    fn runtime_path(&self, name: &str) -> Option<PathBuf> {
        let lockfile = if let Some(ref l) = self.lockfile {
            l.clone()
        } else {
            let lock_path = self.workspace_root.join("forge.lock");
            if lock_path.exists() {
                crate::load_lockfile(&lock_path).unwrap_or_default()
            } else {
                crate::Lockfile::default()
            }
        };
        let runtime = lockfile.runtimes.iter().find(|r| r.name == name)?;
        Some(self.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted"))
    }
}


pub struct SimplePlan {
    pub name: String,
    pub details: serde_json::Value,
}

impl Plan for SimplePlan {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "name": self.name,
            "details": self.details,
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SyncPlan {
    pub downloads: Vec<RuntimeLock>,
    pub skips: Vec<String>,
}

impl Plan for SyncPlan {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "downloads": self.downloads,
            "skips": self.skips,
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct RepairPlan {
    pub broken_runtimes: Vec<String>,
    pub actions: Vec<String>,
    pub raw_actions: Vec<crate::diagnostics::QuickFixAction>,
}

impl Plan for RepairPlan {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "broken_runtimes": self.broken_runtimes,
            "actions": self.actions,
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// 1. InitOperation
pub struct InitOperation;

impl Operation for InitOperation {
    fn name(&self) -> &str { "init" }
    
    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "init".to_string(),
            details: json!({ "action": "Initialize forge.toml and gitignore" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let mut changes = Vec::new();
        
        let toml_path = ctx.workspace_root.join("forge.toml");
        if !toml_path.exists() {
            std::fs::write(&toml_path, "[runtimes]\n")
                .map_err(|e| format!("Failed to create forge.toml: {}", e))?;
            changes.push(ChangeRecord {
                path: toml_path.to_string_lossy().to_string(),
                action: "added".to_string(),
            });
        }
        
        if let Ok(_) = crate::cache::append_to_gitignore(&ctx.workspace_root) {
            changes.push(ChangeRecord {
                path: ctx.workspace_root.join(".gitignore").to_string_lossy().to_string(),
                action: "modified".to_string(),
            });
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec!["Forge initialized successfully".to_string()],
        })
        })
    }
}

// 2. ResolveOperation
pub struct ResolveOperation;

impl Operation for ResolveOperation {
    fn name(&self) -> &str { "resolve" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "resolve".to_string(),
            details: json!({ "action": "Resolve versions from registry" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let config = ctx.load_config()?.clone();
        let resolver = crate::resolver::Resolver::new();
        let platform = crate::registry::detect_platform();
        let arch = crate::registry::detect_arch();
        
        let registry_path = ctx.workspace_root.join(".forge").join("metadata_cache.toml");
        let registry = if registry_path.exists() {
            crate::registry::HybridRegistry::load_from_file(&registry_path)?
        } else {
            crate::registry::HybridRegistry::default_with_internal()
        };

        let mut diagnostics = Vec::new();
        for (name, req) in &config.runtimes {
            let resolved = resolver.resolve(name, req, platform, arch, &registry)?;
            diagnostics.push(format!("Resolved {} v{} -> {}", name, resolved.version, resolved.url));
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes: Vec::new(),
            diagnostics,
        })
        })
    }
}

// 3. LockOperation
pub struct LockOperation;

impl Operation for LockOperation {
    fn name(&self) -> &str { "lock" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "lock".to_string(),
            details: json!({ "action": "Generate or update forge.lock" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let toml_path = ctx.workspace_root.join("forge.toml");
        let lockfile_path = ctx.workspace_root.join("forge.lock");
        
        if !toml_path.exists() {
            return Err("forge.toml not found".to_string());
        }

        let is_new = !lockfile_path.exists();
        let lockfile = crate::update_lockfile(&toml_path, &lockfile_path).await?;
        ctx.lockfile = Some(lockfile);

        let mut changes = Vec::new();
        changes.push(ChangeRecord {
            path: lockfile_path.to_string_lossy().to_string(),
            action: if is_new { "added".to_string() } else { "modified".to_string() },
        });

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec!["Lockfile updated successfully".to_string()],
        })
        })
    }
}

// 4. SyncOperation
pub struct SyncOperation;

impl Operation for SyncOperation {
    fn name(&self) -> &str { "sync" }

    fn plan(&self, ctx: &Context) -> Result<Box<dyn Plan>, String> {
        let mut temp_ctx = Context {
            workspace_root: ctx.workspace_root.clone(),
            cache_dir: ctx.cache_dir.clone(),
            event_bus: ctx.event_bus.clone(),
            operation_id: ctx.operation_id.clone(),
            config: ctx.config.clone(),
            lockfile: ctx.lockfile.clone(),
        };
        let lockfile = temp_ctx.load_lockfile()?.clone();
        let mut downloads = Vec::new();
        let mut skips = Vec::new();

        for runtime in &lockfile.runtimes {
            let target_extract_to = ctx.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
            let mut is_installed = false;
            if target_extract_to.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&target_extract_to) {
                    if entries.next().is_some() {
                        is_installed = true;
                    }
                }
            }
            if is_installed {
                skips.push(runtime.name.clone());
            } else {
                downloads.push(runtime.clone());
            }
        }

        Ok(Box::new(SyncPlan { downloads, skips }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let sync_plan = plan.as_any().downcast_ref::<SyncPlan>()
            .ok_or_else(|| "Invalid plan type for SyncOperation".to_string())?;

        if sync_plan.downloads.is_empty() {
            return Ok(OperationResult {
                status: OperationStatus::Skipped,
                duration_ms: start.elapsed().as_millis() as u64,
                warnings: Vec::new(),
                changes: Vec::new(),
                diagnostics: vec!["All runtimes are already synced".to_string()],
            });
        }

        let mut changes = Vec::new();
        for runtime in &sync_plan.downloads {
            let item_changes = crate::installer::install_runtime_transactional(
                runtime,
                &ctx.workspace_root,
                &ctx.cache_dir,
                &ctx.operation_id,
                Some(&ctx.event_bus),
            ).await?;
            changes.extend(item_changes);
        }

        // Regenerate shims cache upon successful commit of all runtimes
        let lockfile = ctx.load_lockfile()?.clone();
        if let Err(e) = regenerate_shims_cache(&lockfile, &ctx.cache_dir, &ctx.workspace_root) {
            println!("shims cache error: {}", e);
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec![format!("Successfully synced {} runtimes", sync_plan.downloads.len())],
        })
        })
    }
}

// 5. GcOperation
pub struct GcOperation;

impl Operation for GcOperation {
    fn name(&self) -> &str { "gc" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "gc".to_string(),
            details: json!({ "action": "Garbage collect unused runtimes" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let lockfile = ctx.load_lockfile()?.clone();
        
        let mut referenced = std::collections::HashSet::new();
        for runtime in &lockfile.runtimes {
            referenced.insert((runtime.name.clone(), runtime.version.clone()));
        }

        let mut changes = Vec::new();
        if ctx.cache_dir.exists() {
            let names = std::fs::read_dir(&ctx.cache_dir)
                .map_err(|e| format!("Failed to read cache dir: {}", e))?;
            for name_entry in names.flatten() {
                if name_entry.path().is_dir() {
                    let name = name_entry.file_name().to_string_lossy().into_owned();
                    let versions = std::fs::read_dir(name_entry.path())
                        .map_err(|e| format!("Failed to read version dir: {}", e))?;
                    for ver_entry in versions.flatten() {
                        if ver_entry.path().is_dir() {
                            let version = ver_entry.file_name().to_string_lossy().into_owned();
                            if !referenced.contains(&(name.clone(), version.clone())) {
                                let path_to_remove = ver_entry.path();
                                let path_str = path_to_remove.to_string_lossy().into_owned();
                                std::fs::remove_dir_all(&path_to_remove)
                                    .map_err(|e| format!("Failed to delete garbage runtime {}: {}", path_str, e))?;
                                changes.push(ChangeRecord {
                                    path: path_str,
                                    action: "deleted".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec!["Garbage collection complete".to_string()],
        })
        })
    }
}

// 6. CleanOperation
pub struct CleanOperation;

impl Operation for CleanOperation {
    fn name(&self) -> &str { "clean" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "clean".to_string(),
            details: json!({ "action": "Wipe cache, staging, and backup directories" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let mut changes = Vec::new();

        if ctx.cache_dir.exists() {
            let path_str = ctx.cache_dir.to_string_lossy().into_owned();
            std::fs::remove_dir_all(&ctx.cache_dir)
                .map_err(|e| format!("Failed to clear cache directory: {}", e))?;
            changes.push(ChangeRecord {
                path: path_str,
                action: "deleted".to_string(),
            });
        }

        let forge_internal = ctx.workspace_root.join(".forge");
        if forge_internal.exists() {
            let path_str = forge_internal.to_string_lossy().into_owned();
            std::fs::remove_dir_all(&forge_internal)
                .map_err(|e| format!("Failed to clean .forge directory: {}", e))?;
            changes.push(ChangeRecord {
                path: path_str,
                action: "deleted".to_string(),
            });
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec!["Cache wiped clean successfully".to_string()],
        })
        })
    }
}

// 7. RunOperation
pub struct RunOperation {
    pub cmd: String,
    pub args: Vec<String>,
}

impl Operation for RunOperation {
    fn name(&self) -> &str { "run" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "run".to_string(),
            details: json!({ "cmd": self.cmd, "args": self.args }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let lockfile = ctx.load_lockfile()?.clone();
        
        let mut bin_dirs = Vec::new();
        for runtime in &lockfile.runtimes {
            let extract_dir = ctx.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
            let found = find_bin_dirs(&extract_dir);
            bin_dirs.extend(found);
        }

        let _ = ctx.load_config();
        let resolved_env = crate::environment::materialize_environment(ctx, &HashMap::new(), None)?;
        let env_vars = resolved_env.vars;

        let exit_code = crate::launcher::run_command_in_env(&self.cmd, &self.args, &env_vars, &bin_dirs)?;
        let status = if exit_code == 0 {
            OperationStatus::Success
        } else {
            OperationStatus::Failure
        };

        Ok(OperationResult {
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes: Vec::new(),
            diagnostics: vec![format!("Command exited with status code: {}", exit_code)],
        })
        })
    }
}

// 8. ShellOperation
pub struct ShellOperation;

impl Operation for ShellOperation {
    fn name(&self) -> &str { "shell" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "shell".to_string(),
            details: json!({ "action": "Spawn interactive subshell" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let lockfile = ctx.load_lockfile()?.clone();

        let mut bin_dirs = Vec::new();
        for runtime in &lockfile.runtimes {
            let extract_dir = ctx.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
            let found = find_bin_dirs(&extract_dir);
            bin_dirs.extend(found);
        }

        let _ = ctx.load_config();
        let resolved_env = crate::environment::materialize_environment(ctx, &HashMap::new(), None)?;
        let env_vars = resolved_env.vars;

        let exit_code = crate::launcher::spawn_shell_in_env(&env_vars, &bin_dirs)?;
        let status = if exit_code == 0 {
            OperationStatus::Success
        } else {
            OperationStatus::Failure
        };

        Ok(OperationResult {
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes: Vec::new(),
            diagnostics: vec![format!("Shell exited with code {}", exit_code)],
        })
        })
    }
}

// 9. ValidateOperation
pub struct ValidateOperation;

impl Operation for ValidateOperation {
    fn name(&self) -> &str { "validate" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "validate".to_string(),
            details: json!({ "action": "Validate forge.toml and forge.lock schemas" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let mut diagnostics = Vec::new();
        let mut warnings = Vec::new();

        let toml_path = ctx.workspace_root.join("forge.toml");
        if !toml_path.exists() {
            diagnostics.push("forge.toml is missing".to_string());
            return Ok(OperationResult {
                status: OperationStatus::Failure,
                duration_ms: start.elapsed().as_millis() as u64,
                warnings,
                changes: Vec::new(),
                diagnostics,
            });
        }

        match load_config(&toml_path) {
            Ok(config) => {
                diagnostics.push(format!("forge.toml parsed correctly with {} runtimes", config.runtimes.len()));
            }
            Err(e) => {
                diagnostics.push(format!("forge.toml validation error: {}", e));
                return Ok(OperationResult {
                    status: OperationStatus::Failure,
                    duration_ms: start.elapsed().as_millis() as u64,
                    warnings,
                    changes: Vec::new(),
                    diagnostics,
                });
            }
        }

        let lock_path = ctx.workspace_root.join("forge.lock");
        if lock_path.exists() {
            match load_lockfile(&lock_path) {
                Ok(lockfile) => {
                    diagnostics.push(format!("forge.lock validated with {} locked runtimes", lockfile.runtimes.len()));
                }
                Err(e) => {
                    diagnostics.push(format!("forge.lock validation error: {}", e));
                    return Ok(OperationResult {
                        status: OperationStatus::Failure,
                        duration_ms: start.elapsed().as_millis() as u64,
                        warnings,
                        changes: Vec::new(),
                        diagnostics,
                    });
                }
            }
        } else {
            warnings.push("forge.lock does not exist. Run 'forge lock' to generate it".to_string());
        }

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings,
            changes: Vec::new(),
            diagnostics,
        })
        })
    }
}

// 10. PlanOperation
pub struct PlanOperation;

impl Operation for PlanOperation {
    fn name(&self) -> &str { "plan" }

    fn plan(&self, _ctx: &Context) -> Result<Box<dyn Plan>, String> {
        Ok(Box::new(SimplePlan {
            name: "plan".to_string(),
            details: json!({ "action": "Plan operations without executing" }),
        }))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, _plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        
        let sync_op = SyncOperation;
        let sync_plan = sync_op.plan(ctx)?;
        let sync_json = sync_plan.to_json();

        Ok(OperationResult {
            status: OperationStatus::Success,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes: Vec::new(),
            diagnostics: vec![format!("Calculated Sync Plan:\n{}", serde_json::to_string_pretty(&sync_json).unwrap())],
        })
        })
    }
}

pub struct RepairPlanner;

impl RepairPlanner {
    pub fn plan(findings: &[crate::diagnostics::Finding]) -> RepairPlan {
        let mut broken_runtimes = Vec::new();
        let mut actions = Vec::new();
        let mut raw_actions = Vec::new();
        
        for finding in findings {
            if let Some(ref quick_fix) = finding.suggested_quick_fix {
                raw_actions.push(quick_fix.action.clone());
                match &quick_fix.action {
                    crate::diagnostics::QuickFixAction::WipeAndReextract { runtime_name, version } => {
                        let action_str = format!("Re-extract {} v{}", runtime_name, version);
                        if !broken_runtimes.contains(runtime_name) {
                            broken_runtimes.push(runtime_name.clone());
                        }
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::RecreateShim { shim_name } => {
                        let action_str = format!("Recreate shim {}", shim_name);
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::SetEnvVar { key, value } => {
                        let action_str = format!("Set environment variable {} to {}", key, value);
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::SetSecret { key } => {
                        let action_str = format!("Configure secret {}", key);
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::RegenerateLockfile => {
                        let action_str = "Regenerate lockfile".to_string();
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::RegenerateShimsCache => {
                        let action_str = "Regenerate shims cache".to_string();
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                    crate::diagnostics::QuickFixAction::AddToGitIgnore { path } => {
                        let action_str = format!("Add {} to .gitignore", path);
                        if !actions.contains(&action_str) {
                            actions.push(action_str);
                        }
                    }
                }
            }
        }
        
        RepairPlan { broken_runtimes, actions, raw_actions }
    }
}

// 11. RepairOperation
pub struct RepairOperation;

impl Operation for RepairOperation {
    fn name(&self) -> &str { "repair" }

    fn plan(&self, ctx: &Context) -> Result<Box<dyn Plan>, String> {
        let diag_ctx = crate::diagnostics::DiagnosticContext {
            workspace_root: ctx.workspace_root.clone(),
            cache_dir: ctx.cache_dir.clone(),
            mode: crate::diagnostics::DiagnosticMode::Deep,
            active_profile: None,
        };
        
        let report = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        let engine = crate::diagnostics::DiagnosticEngine::new();
                        engine.run(&diag_ctx).await
                    })
                })
            }
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to build tokio runtime: {}", e))?;
                rt.block_on(async {
                    let engine = crate::diagnostics::DiagnosticEngine::new();
                    engine.run(&diag_ctx).await
                })
            }
        };
        
        let plan = RepairPlanner::plan(&report.findings);
        Ok(Box::new(plan))
    }

    fn execute<'a>(&'a self, ctx: &'a mut Context, plan: Box<dyn Plan>) -> OperationFuture<'a> {
        Box::pin(async move {
        let start = std::time::Instant::now();
        let repair_plan = plan.as_any().downcast_ref::<RepairPlan>()
            .ok_or_else(|| "Invalid plan type for RepairOperation".to_string())?;

        let inspect_event = Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: ctx.operation_id.clone(),
            runtime: "all".to_string(),
            phase: "Inspect".to_string(),
            status: EventStatus::Started,
            message: Some("Phase 1: Inspecting cache and directory layout".to_string()),
        };
        let _ = ctx.event_bus.publish(inspect_event);

        // 1. Inspect & 2. Diagnose
        let broken = repair_plan.broken_runtimes.clone();

        let diag_event = Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: ctx.operation_id.clone(),
            runtime: "all".to_string(),
            phase: "Diagnose".to_string(),
            status: EventStatus::Progress(20),
            message: Some(format!("Phase 2: Diagnosed {} broken/missing runtimes", broken.len())),
        };
        let _ = ctx.event_bus.publish(diag_event);

        // 3. Plan (done)
        let plan_event = Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: ctx.operation_id.clone(),
            runtime: "all".to_string(),
            phase: "Plan".to_string(),
            status: EventStatus::Progress(40),
            message: Some(format!("Phase 3: Formulating Repair Plan for {:?}", broken)),
        };
        let _ = ctx.event_bus.publish(plan_event);

        // 4. Repair
        let lockfile = ctx.load_lockfile()?.clone();
        let mut changes = Vec::new();
        
        for name in &broken {
            if let Some(runtime) = lockfile.runtimes.iter().find(|r| &r.name == name) {
                let target_dir = ctx.cache_dir.join(&runtime.name).join(&runtime.version);
                let target_extract_to = target_dir.join("extracted");
                
                // For repair, explicitly wipe target to ensure clean re-install
                if target_extract_to.exists() {
                    let _ = std::fs::remove_dir_all(&target_extract_to);
                }

                let repair_event = Event {
                    timestamp: "2026-07-01T09:24:30-05:00".to_string(),
                    operation_id: ctx.operation_id.clone(),
                    runtime: name.clone(),
                    phase: "Repairing".to_string(),
                    status: EventStatus::Progress(60),
                    message: Some(format!("Phase 4: Running clean staging-promotion for {}", name)),
                };
                let _ = ctx.event_bus.publish(repair_event);

                let item_changes = crate::installer::install_runtime_transactional(
                    runtime,
                    &ctx.workspace_root,
                    &ctx.cache_dir,
                    &ctx.operation_id,
                    Some(&ctx.event_bus),
                ).await?;
                changes.extend(item_changes);
            }
        }

        // 5. Verify
        let verify_event = Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: ctx.operation_id.clone(),
            runtime: "all".to_string(),
            phase: "Verify".to_string(),
            status: EventStatus::Progress(90),
            message: Some("Phase 5: Verifying installation integrity".to_string()),
        };
        let _ = ctx.event_bus.publish(verify_event);

        let mut verify_success = true;
        for name in &broken {
            if let Some(runtime) = lockfile.runtimes.iter().find(|r| &r.name == name) {
                let target_extract_to = ctx.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
                if !target_extract_to.exists() {
                    verify_success = false;
                }
            }
        }

        let status = if verify_success {
            let verify_finished_event = Event {
                timestamp: "2026-07-01T09:24:30-05:00".to_string(),
                operation_id: ctx.operation_id.clone(),
                runtime: "all".to_string(),
                phase: "Verify".to_string(),
                status: EventStatus::Finished,
                message: Some("All runtimes verified successfully".to_string()),
            };
            let _ = ctx.event_bus.publish(verify_finished_event);
            OperationStatus::Success
        } else {
            let verify_failed_event = Event {
                timestamp: "2026-07-01T09:24:30-05:00".to_string(),
                operation_id: ctx.operation_id.clone(),
                runtime: "all".to_string(),
                phase: "Verify".to_string(),
                status: EventStatus::Failed("Verification failed".to_string()),
                message: Some("Repair verification phase failed".to_string()),
            };
            let _ = ctx.event_bus.publish(verify_failed_event);
            OperationStatus::Failure
        };

        // Execute other quick fixes
        for action in &repair_plan.raw_actions {
            match action {
                crate::diagnostics::QuickFixAction::RegenerateLockfile => {
                    let toml_path = ctx.workspace_root.join("forge.toml");
                    let lockfile_path = ctx.workspace_root.join("forge.lock");
                    let _ = crate::update_lockfile(&toml_path, &lockfile_path).await;
                }
                crate::diagnostics::QuickFixAction::RegenerateShimsCache => {
                    let lockfile_val = ctx.load_lockfile().cloned().unwrap_or_default();
                    let _ = crate::cache::regenerate_shims_cache(&lockfile_val, &ctx.cache_dir, &ctx.workspace_root);
                }
                crate::diagnostics::QuickFixAction::AddToGitIgnore { path: _ } => {
                    let _ = crate::cache::append_to_gitignore(&ctx.workspace_root);
                }
                crate::diagnostics::QuickFixAction::SetEnvVar { key, value } => {
                    if let Some(env_path) = crate::environment::find_forge_env(&ctx.workspace_root)
                        .or_else(|| Some(ctx.workspace_root.join("forge.env")))
                    {
                        let content = std::fs::read_to_string(&env_path).unwrap_or_default();
                        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let mut found = false;
                        for line in &mut lines {
                            if line.starts_with(key) && line.contains('=') {
                                *line = format!("{} = \"{}\"", key, value);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            lines.push(format!("{} = \"{}\"", key, value));
                        }
                        let _ = std::fs::write(&env_path, lines.join("\n"));
                    }
                }
                _ => {}
            }
        }

        // Regenerate shims cache
        let _ = regenerate_shims_cache(&lockfile, &ctx.cache_dir, &ctx.workspace_root);

        Ok(OperationResult {
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            warnings: Vec::new(),
            changes,
            diagnostics: vec![format!("Repair operation completed with status: {:?}", status)],
        })
        })
    }
}


