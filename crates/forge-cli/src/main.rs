use std::collections::HashMap;
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use serde_json::json;
use forge_core::operations::{
    InitOperation, ResolveOperation, LockOperation, GcOperation,
    RunOperation, ShellOperation, PlanOperation, ValidateOperation,
};
use forge_core::api::v1::OperationSummary;
use forge_core::context::{ContextEngine, ContextOptions, ForgeContext};
use forge_core::manifest::ForgeConfig;
use forge_core::secrets::{ResolvedEnvironment, ValueSource};
use forge_core::{CliCommand, ContextExporter, AgentAdapter, PluginRegistry, PolicyEngine};

mod benchmark;
mod jsonrpc;
mod mcp;

/// Built-in command names that take precedence over plugin commands.
const BUILTIN_COMMANDS: &[&str] = &[
    "init", "resolve", "lock", "sync", "up", "run", "shell",
    "clean", "gc", "status", "inspect", "repair", "plan",
    "history", "explain", "trace", "events", "setup", "doctor",
    "which", "ai", "env", "secret", "context",
    "bundle", "restore", "snapshot",
    "benchmark",
];

#[derive(Parser)]
#[command(name = "forge", version = "0.1.0", about = "Reproducible development environments — for humans, tools, and AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Initialize forge in the current directory")]
    Init,
    #[command(about = "Resolve runtime versions")]
    Resolve,
    #[command(about = "Generate or update forge.lock")]
    Lock,
    #[command(about = "Sync runtimes from lockfile")]
    Sync,
    #[command(about = "Sync runtimes (resolve, lock, sync combined)")]
    Up,
    #[command(about = "Execute a command inside the activated environment")]
    Run {
        cmd: String,
        args: Vec<String>,
    },
    #[command(about = "Spawn an interactive subshell inside the environment")]
    Shell,
    #[command(about = "Clean all local cache and state")]
    Clean,
    #[command(about = "Garbage collect unused runtimes")]
    Gc,
    #[command(about = "Show current environment lifecycle status")]
    Status,
    #[command(about = "Inspect environment health and runtimes")]
    Inspect,
    #[command(about = "Repair corrupted or missing runtimes")]
    Repair,
    #[command(about = "Show proposed changes plan")]
    Plan,
    #[command(about = "Show past operations history")]
    History {
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "table")]
        format: String,
    },
    #[command(about = "Display resolved configuration, cache and shims for a runtime, or inspect operations, context, config, and profile details")]
    Explain {
        args: Vec<String>,
    },
    #[command(about = "Show operation hierarchy and durations")]
    Trace {
        op_id: String,
    },
    #[command(about = "Stream operation events journal")]
    Events {
        #[arg(long)]
        live: bool,
    },

    // Keep these for backward compatibility/tests:
    #[command(about = "Setup shims")]
    Setup {
        #[arg(long)]
        uninstall: bool,
    },
    #[command(about = "Doctor checks")]
    Doctor {
        #[arg(long, help = "Run cryptographic and process checks")]
        deep: bool,
        #[arg(long, help = "Format output as JSON")]
        json: bool,
    },
    #[command(about = "Locate a runtime")]
    Which {
        runtime: String,
    },
    #[command(about = "AI checks")]
    Ai {
        #[command(subcommand)]
        subcommand: AiCommands,
    },
    #[command(about = "Manage environment configurations")]
    Env {
        #[command(subcommand)]
        subcommand: EnvCommands,
    },
    #[command(about = "Manage secure credentials and secrets")]
    Secret {
        #[command(subcommand)]
        subcommand: SecretCommands,
    },
    #[command(about = "Create a portable .forge archive of forge descriptors")]
    Bundle {
        #[arg(long, help = "Custom output path (default: <project_dir_name>.forge)")]
        output: Option<PathBuf>,
    },
    #[command(about = "Restore forge descriptors from a .forge archive")]
    Restore {
        path: PathBuf,
        #[arg(long, help = "Overwrite existing files without prompting")]
        force: bool,
    },
    /// Catch-all for plugin CLI commands
    #[command(external_subcommand)]
    PluginCommand(Vec<String>),

    #[command(name = "jsonrpc", about = "Start JSON-RPC 2.0 server over stdin/stdout for SDK transport")]
    JsonRpc,

    #[command(name = "mcp", about = "Start MCP (Model Context Protocol) server over stdin/stdout for AI agent integration")]
    Mcp,

    #[command(name = "tui", about = "Launch terminal dashboard")]
    Tui,

    #[command(about = "Manage remote registry and FRRS metadata cache")]
    Registry {
        #[command(subcommand)]
        subcommand: RegistryCommands,
    },

    #[command(about = "Manage environment snapshots")]
    Snapshot {
        #[command(subcommand)]
        subcommand: SnapshotCommands,
    },

    #[command(about = "Run engine benchmarks and report performance metrics")]
    Benchmark {
        #[arg(long, help = "Format output as JSON")]
        json: bool,
        #[arg(long, help = "Compare results with previous run (future)" )]
        compare: bool,
    },

    #[command(about = "Display active environment configuration and workspace details")]
    Context {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long, value_delimiter = ',')]
        scope: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        exclude: Vec<String>,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    #[command(about = "List environment variables")]
    List,
    #[command(about = "Get environment variable value")]
    Get {
        key: String,
    },
    #[command(about = "Set environment variable value")]
    Set {
        key: String,
        value: String,
    },
    #[command(about = "Unset/remove environment variable")]
    Unset {
        key: String,
    },
    #[command(about = "Resolve the unified environment")]
    Resolve {
        #[arg(long)]
        profile: Option<String>,
    },
}

#[derive(Subcommand)]
enum SecretCommands {
    #[command(about = "Set a secret")]
    Set {
        key: String,
        value: String,
    },
    #[command(about = "Get a secret")]
    Get {
        key: String,
    },
    #[command(about = "List all secrets")]
    List,
    #[command(about = "Remove a secret")]
    Remove {
        key: String,
    },
    #[command(about = "Export all secrets as JSON")]
    Export,
    #[command(about = "Import secrets from JSON file")]
    Import {
        file: PathBuf,
    },
    #[command(about = "Verify OS Keyring connectivity and configuration health")]
    Doctor,
}


#[derive(Subcommand)]
enum RegistryCommands {
    #[command(about = "Clear FRRS metadata cache and re-fetch from remote registry")]
    Refresh,
}

#[derive(Subcommand)]
enum AiCommands {
    #[command(about = "Display active environment configuration and runtimes with masked secrets")]
    Context,
    #[command(about = "Perform diagnostics check and output remediation instructions")]
    Doctor,
}

#[derive(Subcommand)]
enum SnapshotCommands {
    #[command(about = "Create a new environment snapshot")]
    Create {
        #[arg(long, help = "Custom snapshot name (default: auto-generated timestamp)")]
        name: Option<String>,
        #[arg(long, help = "Optional description for the snapshot")]
        description: Option<String>,
    },
    #[command(about = "List all saved snapshots")]
    List,
    #[command(name = "restore", about = "Restore environment state from a snapshot")]
    SnapshotRestore {
        name: String,
        #[arg(long, help = "Preview what would be restored without modifying files")]
        dry_run: bool,
    },
}





async fn run_operation<O: forge_core::operations::Operation + 'static>(
    op: O,
    current_dir: &Path,
    event_bus: &forge_core::event_bus::EventBus,
) -> Result<(), String> {
    let cache_dir = forge_core::get_cache_dir()?;
    let workspace_root = forge_core::find_forge_toml(current_dir)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| current_dir.to_path_buf());

    let mut ctx = forge_core::operations::Context::new(workspace_root.clone(), cache_dir, event_bus.clone());
    
    // Load config and lockfile if available
    let _ = ctx.load_config();
    let _ = ctx.load_lockfile();

    let plan = op.plan(&ctx)?;
    let result = op.execute(&mut ctx, plan).await?;

    // Print result diagnostics
    for diag in &result.diagnostics {
        println!("{}", diag);
    }
    for warn in &result.warnings {
        println!("WARNING: {}", warn);
    }
    
    // Print changes
    for change in &result.changes {
        println!("  {} -> {}", change.action, change.path);
    }

    // Save final computed state
    let final_state = forge_core::compute_current_state(&workspace_root, &ctx.cache_dir);
    forge_core::save_state(&workspace_root, final_state);

    if result.status == forge_core::types::OperationStatus::Failure {
        return Err(format!("Operation '{}' failed", op.name()));
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_cli(cli).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_history_table(history: &[forge_core::api::v1::OperationSummary]) {
    println!("{:<20} | {:<12} | {:<15} | {:<10}", "Operation ID", "Runtime", "Duration (ms)", "Status");
    println!("{}", "-".repeat(67));
    for op in history {
        println!("{:<20} | {:<12} | {:<15} | {:<10}", op.id, op.runtime, op.duration_ms, op.status);
    }
}

fn print_explain_table(exp: &forge_core::api::v1::RuntimeExplanation) {
    println!("{:<20} | {:<50}", "Property", "Value");
    println!("{}", "-".repeat(73));
    println!("{:<20} | {:<50}", "Runtime", exp.runtime);
    println!("{:<20} | {:<50}", "State", exp.state);
    println!("{}", "-".repeat(73));
    println!("Diagnostics:");
    for diag in &exp.diagnostics {
        println!("  - {}", diag);
    }
}

// ── Explain Subcommand ─────────────────────────────────────────────────

/// Type-safe enum for forge explain subcommands.
/// Parsed manually from Vec<String> to maintain backward compat
/// with `forge explain <runtime>` (no subcommand keyword needed).
#[derive(Debug)]
enum ExplainSubcommand {
    Runtime { name: String },
    Operation { id: String },
    Context,
    Config,
    Profile,
}

impl ExplainSubcommand {
    fn parse(args: &[String]) -> Result<Self, String> {
        match args.first() {
            Some(s) if s == "operation" || s == "op" => {
                let id = args.get(1).ok_or_else(|| {
                    "Usage: forge explain operation <id>".to_string()
                })?;
                Ok(Self::Operation { id: id.clone() })
            }
            Some(s) if s == "context" || s == "ctx" => Ok(Self::Context),
            Some(s) if s == "config" || s == "cfg" => Ok(Self::Config),
            Some(s) if s == "profile" || s == "prof" => Ok(Self::Profile),
            Some(name) => Ok(Self::Runtime { name: name.clone() }),
            None => Err(
                "Missing runtime name or subcommand. Usage: forge explain <runtime> [subcommand]"
                    .to_string(),
            ),
        }
    }
}

/// Top-level dispatch for `forge explain ...`.
/// Parses subcommand from args, then delegates to per-variant handlers.
async fn handle_explain(current_dir: &Path, args: &[String]) -> Result<(), String> {
    let subcommand = ExplainSubcommand::parse(args)?;
    match subcommand {
        ExplainSubcommand::Runtime { name } => {
            let engine = forge_core::Engine::new(current_dir.to_path_buf())?;
            let explanation = engine.explain(&name).await?;
            print_explain_table(&explanation);
        }
        ExplainSubcommand::Operation { id } => {
            let engine = forge_core::Engine::new(current_dir.to_path_buf())?;
            explain_operation(&engine, &id).await?;
        }
        ExplainSubcommand::Context => {
            explain_context(current_dir).await?;
        }
        ExplainSubcommand::Config => {
            let engine = forge_core::Engine::new(current_dir.to_path_buf())?;
            explain_config(&engine, current_dir).await?;
        }
        ExplainSubcommand::Profile => {
            explain_profile(current_dir).await?;
        }
    }
    Ok(())
}

// ── Explain: Operation handler ─────────────────────────────────────────

async fn explain_operation(engine: &forge_core::Engine, id: &str) -> Result<(), String> {
    let history = engine.history(None).await?;
    let op = history
        .iter()
        .find(|o| o.id == id)
        .ok_or_else(|| format!("Operation '{}' not found in history", id))?;
    let trace = engine.trace(id).await?;
    print_operation_table(op, &trace);
    Ok(())
}

fn print_operation_table(summary: &OperationSummary, trace: &str) {
    println!("{:<20} | {:<50}", "Property", "Value");
    println!("{}", "-".repeat(73));
    println!("{:<20} | {:<50}", "Operation ID", summary.id);
    println!("{:<20} | {:<50}", "Runtime", summary.runtime);
    println!("{:<20} | {:<50}", "Duration (ms)", summary.duration_ms.to_string());
    println!("{:<20} | {:<50}", "Status", summary.status);
    println!("{}", "-".repeat(73));
    println!("Event Timeline:");
    println!("{}", trace);
}

// ── Explain: Context handler ───────────────────────────────────────────

async fn explain_context(current_dir: &Path) -> Result<(), String> {
    let mut engine = ContextEngine::new();
    engine.register(std::sync::Arc::new(forge_core::RuntimeProviderImpl));
    engine.register(std::sync::Arc::new(forge_core::ConfigurationProviderImpl));
    engine.register(std::sync::Arc::new(forge_core::DiagnosticsProviderImpl));
    engine.register(std::sync::Arc::new(forge_core::WorkspaceProviderImpl));
    engine.register(std::sync::Arc::new(forge_core::EnvironmentProviderImpl));
    engine.register(std::sync::Arc::new(forge_core::SecretsProviderImpl));

    let cache_dir = forge_core::get_cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {}", e))?;

    let toml_path = forge_core::find_forge_toml(current_dir);
    let active_profile = if let Some(ref path) = toml_path {
        std::env::var("FORGE_PROFILE").ok().or_else(|| {
            forge_core::load_config(path).ok().and_then(|c| {
                c.profile.and_then(|p| p.keys().next().cloned())
            })
        })
    } else {
        None
    };

    let options = ContextOptions {
        scopes: Vec::new(),
        excludes: Vec::new(),
        workspace_root: current_dir.to_path_buf(),
        cache_dir,
        active_profile,
    };

    let context = engine.query(&options).await?;
    print_context_table(&context);
    Ok(())
}

fn print_context_table(ctx: &ForgeContext) {
    println!("{:<25} | {:<50}", "Provider", "Status");
    println!("{}", "-".repeat(78));

    let providers = [
        ("runtime", &ctx.runtimes),
        ("configuration", &ctx.config),
        ("diagnostics", &ctx.diagnostics),
        ("workspace", &ctx.workspace),
        ("environment", &ctx.environment),
        ("secrets", &ctx.secrets_metadata),
    ];

    for (name, data) in &providers {
        let has_error = data.get("error").and_then(|v| v.as_str()).is_some();
        let status = if has_error {
            format!("Error: {}", data["error"].as_str().unwrap_or("unknown"))
        } else if data.is_null() {
            "Skipped".to_string()
        } else {
            "Collected".to_string()
        };
        println!("{:<25} | {:<50}", name, status);
    }

    // Show workspace limits
    if let Some(workspace) = ctx.workspace.as_object() {
        println!("{}", "-".repeat(78));
        if let Some(files) = workspace.get("total_files") {
            println!("{:<25} | {:<50}", "Workspace files", files);
        }
        if let Some(depth) = workspace.get("max_depth") {
            println!("{:<25} | {:<50}", "Workspace max depth", depth);
        }
    }

    // Show masked secrets metadata
    if let Some(secrets) = ctx.secrets_metadata.as_object() {
        if !secrets.is_empty() {
            println!("{}", "-".repeat(78));
            println!("{:<25} | {:<50}", "Secrets", "[MASKED]");
            for (key, _val) in secrets {
                println!("{:<25} | {:<50}", format!("  .{}", key), "[MASKED]");
            }
        }
    }
}

// ── Explain: Config handler ────────────────────────────────────────────

async fn explain_config(engine: &forge_core::Engine, current_dir: &Path) -> Result<(), String> {
    let resolved = engine.env_resolve(None).await?;

    let toml_path = forge_core::find_forge_toml(current_dir);
    let has_secret_defs = toml_path.as_ref().and_then(|p| {
        forge_core::load_config(p).ok().and_then(|c| c.config)
    });

    print_config_table(&resolved, has_secret_defs.as_ref());
    Ok(())
}

fn print_config_table(resolved: &ResolvedEnvironment, config_section: Option<&forge_core::manifest::ConfigSection>) {
    let secret_keys: std::collections::HashSet<&str> = config_section
        .map(|cs| {
            cs.definitions
                .iter()
                .filter(|(_, def)| def.secret)
                .map(|(k, _)| k.as_str())
                .collect()
        })
        .unwrap_or_default();

    println!("{:<25} | {:<25} | {:<50}", "Variable", "Source", "Value");
    println!("{}", "-".repeat(103));

    // Collect all var keys, preserving order from metadata if available
    let mut keys: Vec<&String> = resolved.metadata.keys().collect();
    // Add any vars not in metadata
    for k in resolved.vars.keys() {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }

    for key in &keys {
        let source = resolved.metadata.get(*key).map(|m| match &m.source {
            ValueSource::CliOverride => "CLI Override",
            ValueSource::SystemEnv => "System Env",
            ValueSource::LocalOverride => "Local Override",
            ValueSource::SecretProvider(_) => "Secret Provider",
            ValueSource::EnvFile => "Env File",
            ValueSource::ProfileOverlay(_) => "Profile Overlay",
            ValueSource::DefaultManifest => "Default",
        }).unwrap_or("Unknown");

        let value = if secret_keys.contains(key.as_str()) {
            "[MASKED]".to_string()
        } else {
            resolved.vars.get(*key).cloned().unwrap_or_default()
        };

        println!("{:<25} | {:<25} | {:<50}", *key, source, value);
    }
}

// ── Explain: Profile handler ───────────────────────────────────────────

async fn explain_profile(current_dir: &Path) -> Result<(), String> {
    let toml_path = forge_core::find_forge_toml(current_dir)
        .ok_or_else(|| "No forge.toml found".to_string())?;
    let config = forge_core::load_config(&toml_path)?;

    let active_name = get_active_profile(current_dir)
        .ok_or_else(|| "No active profile found. Set FORGE_PROFILE or add a [profile] section to forge.toml".to_string())?;

    let engine = forge_core::Engine::new(current_dir.to_path_buf())?;
    let resolved = engine.env_resolve(Some(&active_name)).await?;

    print_profile_table(&active_name, &config, &resolved);
    Ok(())
}

fn print_profile_table(active_name: &str, config: &ForgeConfig, resolved: &ResolvedEnvironment) {
    println!("Active Profile: {}", active_name);
    println!("{}", "-".repeat(80));

    let profile_vars: HashMap<String, String> = config
        .profile
        .as_ref()
        .and_then(|p| p.get(active_name))
        .map(|section| {
            section
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if profile_vars.is_empty() {
        println!("No profile variables defined for '{}'.", active_name);
        return;
    }

    println!("{:<25} | {:<30} | {:<25}", "Variable", "Profile Value", "Override Status");
    println!("{}", "-".repeat(83));

    for (key, profile_val) in &profile_vars {
        let _resolved_val = resolved.vars.get(key);
        let source = resolved.metadata.get(key);

        let status = match source {
            Some(m) if matches!(m.source, ValueSource::ProfileOverlay(_)) => {
                "active".to_string()
            }
            Some(m) => format!("overridden by {:?}", m.source),
            None => "active (not resolved)".to_string(),
        };

        println!("{:<25} | {:<30} | {:<25}", key, profile_val, status);
    }
}

async fn run_cli(cli: Cli) -> Result<(), String> {
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current working directory: {}", e))?;
    
    // ── Plugin loading ──────────────────────────────────────────────────
    // Create a registry, register any workspace-member plugins, resolve
    // dependencies, and collect extensions for dispatch.
    let mut plugin_registry = PluginRegistry::new();
    // Future: register workspace-member plugins here:
    //   plugin_registry.register(Box::new(my_plugin::MyPlugin))?;
    if let Err(e) = plugin_registry.resolve_and_init() {
        eprintln!("Warning: Plugin initialization failed: {}", e);
    }

    // Collect plugin extensions before draining CLI commands (they remain
    // in the registry via Arc clones; draining CLI commands is separate).
    let plugin_context_providers = plugin_registry.context_providers();
    let plugin_context_exporters = plugin_registry.context_exporters();
    let plugin_health_checks = plugin_registry.health_checks();

    // Collect plugin CLI commands and reject name conflicts with built-ins
    let all_plugin_commands = plugin_registry.drain_cli_commands();
    let mut plugin_cli_commands: Vec<Box<dyn CliCommand>> = Vec::new();
    for cmd in all_plugin_commands {
        if BUILTIN_COMMANDS.contains(&cmd.name()) {
            eprintln!(
                "Warning: Plugin command '{}' conflicts with built-in command — ignored.",
                cmd.name()
            );
        } else {
            plugin_cli_commands.push(cmd);
        }
    }
    let plugin_cmd_names: Vec<String> =
        plugin_cli_commands.iter().map(|c| c.name().to_string()).collect();
    // ── End plugin loading ──────────────────────────────────────────────

    // Setup event bus and subscribe progress output
    let event_bus = forge_core::event_bus::EventBus::new(100);
    let mut rx = event_bus.subscribe();
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = event.message.unwrap_or_else(|| "Processing".to_string());
            match event.status {
                forge_core::types::EventStatus::Started => {
                    println!("[+] [{}] Starting phase: {}", event.runtime, event.phase);
                }
                forge_core::types::EventStatus::Progress(pct) => {
                    println!("[~] [{}] Progress: {}% - {}", event.runtime, pct, msg);
                }
                forge_core::types::EventStatus::Finished => {
                    println!("[*] [{}] Finished: {}", event.runtime, msg);
                }
                forge_core::types::EventStatus::Failed(err) => {
                    eprintln!("[-] [{}] Failed: {} (Error: {})", event.runtime, msg, err);
                }
            }
        }
    });

    match cli.command {
        Commands::Init => {
            run_operation(InitOperation, &current_dir, &event_bus).await?;
        }
        Commands::Resolve => {
            run_operation(ResolveOperation, &current_dir, &event_bus).await?;
        }
        Commands::Lock => {
            run_operation(LockOperation, &current_dir, &event_bus).await?;
        }
        Commands::Sync => {
            // ── Policy pre-flight check ────────────────────────
            if let Some(engine) = build_policy_engine(&current_dir) {
                let (lockfile_exists, lockfile_has_hashes) = compute_lockfile_state(&current_dir);
                let violations = engine.check_before_sync(lockfile_exists, lockfile_has_hashes);
                enforce_policy(violations);
            }
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.sync().await?;
        }
        Commands::Up => {
            // ── Policy pre-flight check ────────────────────────
            if let Some(engine) = build_policy_engine(&current_dir) {
                let (lockfile_exists, lockfile_has_hashes) = compute_lockfile_state(&current_dir);
                let health_score = compute_health_score(&current_dir, &plugin_health_checks);
                let violations = engine.check_before_up(lockfile_exists, lockfile_has_hashes, health_score);
                enforce_policy(violations);
            }
            println!("Ensuring lockfile is updated...");
            run_operation(LockOperation, &current_dir, &event_bus).await?;
            println!("Syncing runtimes...");
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.sync().await?;
        }
        Commands::Run { cmd, args } => {
            // ── Policy pre-flight check ────────────────────────
            if let Some(engine) = build_policy_engine(&current_dir) {
                let health_score = compute_health_score(&current_dir, &plugin_health_checks);
                let active_profile = get_active_profile(&current_dir);
                let active_runtimes = get_active_runtimes(&current_dir);
                let violations = engine.check_before_run(
                    active_profile.as_deref(),
                    &active_runtimes,
                    health_score,
                );
                enforce_policy(violations);
            }
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.sync().await?;
            run_operation(RunOperation { cmd, args }, &current_dir, &event_bus).await?;
        }
        Commands::Shell => {
            // ── Policy pre-flight check ────────────────────────
            if let Some(engine) = build_policy_engine(&current_dir) {
                let health_score = compute_health_score(&current_dir, &plugin_health_checks);
                let active_profile = get_active_profile(&current_dir);
                let active_runtimes = get_active_runtimes(&current_dir);
                let violations = engine.check_before_run(
                    active_profile.as_deref(),
                    &active_runtimes,
                    health_score,
                );
                enforce_policy(violations);
            }
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.sync().await?;
            run_operation(ShellOperation, &current_dir, &event_bus).await?;
        }
        Commands::Clean => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.clean().await?;
        }
        Commands::Gc => {
            run_operation(GcOperation, &current_dir, &event_bus).await?;
        }
        Commands::Status => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            let status = engine.get_status().await?;
            println!("Lifecycle State: {}", status);
        }
        Commands::Env { subcommand } => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            match subcommand {
                EnvCommands::List => {
                    let vars = engine.env_list().await?;
                    for (k, v) in vars {
                        println!("{}={}", k, v);
                    }
                }
                EnvCommands::Get { key } => {
                    if let Some(val) = engine.env_get(&key).await? {
                        println!("{}", val);
                    }
                }
                EnvCommands::Set { key, value } => {
                    engine.env_set(&key, &value).await?;
                    println!("Set {}={}", key, value);
                }
                EnvCommands::Unset { key } => {
                    engine.env_unset(&key).await?;
                    println!("Unset {}", key);
                }
                EnvCommands::Resolve { profile } => {
                    let resolved = engine.env_resolve(profile.as_deref()).await?;
                    for (k, v) in resolved.vars {
                        println!("{}={}", k, v);
                    }
                }
            }
        }
        Commands::Secret { subcommand } => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            match subcommand {
                SecretCommands::Set { key, value } => {
                    engine.secret_set(&key, &value).await?;
                    println!("Secret '{}' set successfully.", key);
                }
                SecretCommands::Get { key } => {
                    if let Some(val) = engine.secret_get(&key).await? {
                        println!("{}", val);
                    } else {
                        eprintln!("Secret '{}' not found.", key);
                        std::process::exit(1);
                    }
                }
                SecretCommands::List => {
                    let keys = engine.secret_list().await?;
                    for key in keys {
                        println!("{}", key);
                    }
                }
                SecretCommands::Remove { key } => {
                    engine.secret_remove(&key).await?;
                    println!("Secret '{}' removed successfully.", key);
                }
                SecretCommands::Export => {
                    let exported = engine.secret_export().await?;
                    let json_str = serde_json::to_string_pretty(&exported)
                        .map_err(|e| format!("Failed to serialize secrets to JSON: {}", e))?;
                    println!("{}", json_str);
                }
                SecretCommands::Import { file } => {
                    let content = std::fs::read_to_string(&file)
                        .map_err(|e| format!("Failed to read import file: {}", e))?;
                    let secrets: HashMap<String, String> = serde_json::from_str(&content)
                        .map_err(|e| format!("Invalid secrets JSON: {}", e))?;
                    engine.secret_import(&secrets).await?;
                    println!("Successfully imported {} secrets.", secrets.len());
                }
                SecretCommands::Doctor => {
                    let report = engine.secret_doctor().await?;
                    for line in report {
                        println!("{}", line);
                    }
                }
            }
        }
        Commands::Inspect => {
            run_operation(ValidateOperation, &current_dir, &event_bus).await?;
            let workspace_root = forge_core::find_forge_toml(&current_dir)
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| current_dir.clone());
            let shims_cache = workspace_root.join(".forge").join("shims.cache");
            if shims_cache.exists() {
                println!("Shims cache: Present (Path: {})", shims_cache.display());
            } else {
                println!("Shims cache: Missing");
            }
        }
        Commands::Repair => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.repair().await?;
        }
        Commands::Plan => {
            run_operation(PlanOperation, &current_dir, &event_bus).await?;
        }
        Commands::History { limit, format } => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            let history = engine.history(limit).await?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&history).unwrap());
            } else {
                print_history_table(&history);
            }
        }
        Commands::Explain { args } => {
            handle_explain(&current_dir, &args).await?;
        }
        Commands::Trace { op_id } => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            let trace = engine.trace(&op_id).await?;
            println!("{}", trace);
        }
        Commands::Events { live } => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            let mut rx = engine.events(live).await?;
            while let Some(event) = rx.recv().await {
                let msg = event.message.unwrap_or_else(|| "Processing".to_string());
                match event.status {
                    forge_core::types::EventStatus::Started => {
                        println!("[+] [{}] Starting phase: {}", event.runtime, event.phase);
                    }
                    forge_core::types::EventStatus::Progress(pct) => {
                        println!("[~] [{}] Progress: {}% - {}", event.runtime, pct, msg);
                    }
                    forge_core::types::EventStatus::Finished => {
                        println!("[*] [{}] Finished: {}", event.runtime, msg);
                    }
                    forge_core::types::EventStatus::Failed(err) => {
                        eprintln!("[-] [{}] Failed: {} (Error: {})", event.runtime, msg, err);
                    }
                }
            }
        }
        
        Commands::JsonRpc => {
            jsonrpc::serve(current_dir.clone()).await?;
        }
        Commands::Mcp => {
            mcp::serve(current_dir.clone()).await?;
        }
        Commands::Tui => {
            let engine = forge_core::Engine::new(current_dir.clone())?;
            let cache_dir = forge_core::get_cache_dir()?;
            let diag_ctx = forge_core::DiagnosticContext {
                workspace_root: current_dir.clone(),
                cache_dir,
                mode: forge_core::DiagnosticMode::Deep,
                active_profile: None,
            };
            forge_tui::App::run(engine, diag_ctx, plugin_health_checks).await?;
        }
        Commands::Benchmark { json, compare } => {
            benchmark::run_benchmarks(&current_dir, json, compare).await?;
        }
        Commands::Context { format, scope, exclude } => {
            let mut engine = forge_core::ContextEngine::new();
            engine.register(std::sync::Arc::new(forge_core::RuntimeProviderImpl));
            engine.register(std::sync::Arc::new(forge_core::ConfigurationProviderImpl));
            engine.register(std::sync::Arc::new(forge_core::DiagnosticsProviderImpl));
            engine.register(std::sync::Arc::new(forge_core::WorkspaceProviderImpl));
            engine.register(std::sync::Arc::new(forge_core::EnvironmentProviderImpl));
            engine.register(std::sync::Arc::new(forge_core::SecretsProviderImpl));

            // Register plugin context providers and exporters
            engine.register_plugin_providers(plugin_context_providers);
            for exporter in plugin_context_exporters {
                let _ = engine.register_plugin_exporter(exporter);
            }

            let cache_dir = forge_core::get_cache_dir()
                .map_err(|e| format!("Failed to get cache directory: {}", e))?;

            // Retrieve active profile if any
            let toml_path = forge_core::find_forge_toml(&current_dir);
            let active_profile = if let Some(ref path) = toml_path {
                std::env::var("FORGE_PROFILE").ok().or_else(|| {
                    forge_core::load_config(path).ok().and_then(|c| {
                        c.profile.and_then(|p| p.keys().next().cloned())
                    })
                })
            } else {
                None
            };

            let options = forge_core::ContextOptions {
                scopes: scope,
                excludes: exclude,
                workspace_root: current_dir.clone(),
                cache_dir,
                active_profile,
            };

            // Run context engine query
            let context = engine.query(&options).await?;

            // Print formatted output to stdout, and errors to stderr
            match format.as_str() {
                "json" => {
                    let exporter = forge_core::JsonExporter { pretty: false };
                    let output = exporter.export(&context)?;
                    println!("{}", output);
                }
                "json-pretty" | "pretty" => {
                    let exporter = forge_core::JsonExporter { pretty: true };
                    let output = exporter.export(&context)?;
                    println!("{}", output);
                }
                "markdown" | "md" => {
                    let exporter = forge_core::MarkdownExporter;
                    let output = exporter.export(&context)?;
                    println!("{}", output);
                }
                "mcp" => {
                    let exporter = forge_core::McpExporter;
                    let output = exporter.export(&context)?;
                    println!("{}", output);
                }
                "claude" => {
                    let exporter = forge_core::JsonExporter { pretty: false };
                    let adapter = forge_core::ClaudeCodeAdapter;
                    let output = adapter.adapt(&context, &exporter)?;
                    println!("{}", output);
                }
                "gemini" => {
                    let exporter = forge_core::JsonExporter { pretty: false };
                    let adapter = forge_core::GeminiCliAdapter;
                    let output = adapter.adapt(&context, &exporter)?;
                    println!("{}", output);
                }
                "aider" => {
                    let exporter = forge_core::JsonExporter { pretty: false };
                    let adapter = forge_core::AiderAdapter;
                    let output = adapter.adapt(&context, &exporter)?;
                    println!("{}", output);
                }
                "continue" => {
                    let exporter = forge_core::MarkdownExporter;
                    let adapter = forge_core::ContinueAdapter;
                    let output = adapter.adapt(&context, &exporter)?;
                    println!("{}", output);
                }
                other => {
                    return Err(format!("Unsupported format: {}", other));
                }
            }
        }
        Commands::Registry { subcommand } => {
            match subcommand {
                RegistryCommands::Refresh => {
                    let workspace_root = forge_core::find_forge_toml(&current_dir)
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_else(|| current_dir.clone());

                    let frrs_cache_dir = workspace_root.join(".forge").join("metadata_cache");
                    let registry_url = std::env::var("FORGE_REGISTRY_URL")
                        .unwrap_or_else(|_| "https://registry.forge.sh".to_string());

                    if registry_url.is_empty() {
                        eprintln!("Error: Remote registry is disabled (FORGE_REGISTRY_URL is empty).");
                        std::process::exit(1);
                    }

                    // Clear existing cache directory
                    if frrs_cache_dir.exists() {
                        println!("Clearing FRRS metadata cache...");
                        std::fs::remove_dir_all(&frrs_cache_dir)
                            .map_err(|e| format!("Failed to clear cache: {}", e))?;
                    }

                    let remote = forge_core::RemoteRegistry::new(&registry_url, frrs_cache_dir.clone());

                    println!("Fetching registry index from {}...", registry_url);
                    let index = remote.fetch_index().await
                        .map_err(|e| format!("Failed to fetch registry index: {}", e))?;

                    let total = index.toolchains.len();
                    println!("Found {} toolchains in registry. Fetching metadata...", total);

                    for (i, (name, entry)) in index.toolchains.iter().enumerate() {
                        let latest = &entry.latest_version;
                        print!("[{}/{}] Fetching {} {} ... ", i + 1, total, name, latest);
                        match remote.fetch_metadata(name, latest).await {
                            Ok(_) => println!("OK"),
                            Err(e) => eprintln!("FAILED: {}", e),
                        }
                    }

                    println!("\nRegistry refresh complete.");
                }
            }
        }
        Commands::Setup { uninstall } => {
            if uninstall {
                uninstall_shims()?;
            } else {
                setup_shims()?;
            }
        }
        Commands::Doctor { deep, json } => {
            run_doctor(&current_dir, deep, json, &plugin_health_checks)?;
        }
        Commands::Which { runtime } => {
            run_which(&current_dir, &runtime)?;
        }
        Commands::Bundle { output } => {
            let workspace_root = forge_core::find_forge_toml(&current_dir)
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| current_dir.clone());

            let output_path = match output {
                Some(p) => p,
                None => {
                    let dir_name = workspace_root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "project".to_string());
                    workspace_root.join(format!("{}.forge", dir_name))
                }
            };

            forge_core::bundle::create_bundle(&workspace_root, &output_path)
                .map_err(|e| format!("Failed to create bundle: {}", e))?;

            println!("Bundle created: {}", output_path.display());
        }
        Commands::Restore { path, force } => {
            let workspace_root = forge_core::find_forge_toml(&current_dir)
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| current_dir.clone());

            // Check for existing files unless --force is set
            if !force {
                let toml_path = workspace_root.join("forge.toml");
                if toml_path.exists() {
                    return Err("forge.toml already exists. Use --force to overwrite.".to_string());
                }
            }

            forge_core::bundle::restore_bundle(&path, &workspace_root)
                .map_err(|e| format!("Failed to restore bundle: {}", e))?;

            println!("Bundle restored successfully from: {}", path.display());

            // Delegate to `forge up` to download/sync runtimes
            println!("Syncing runtimes...");
            let engine = forge_core::Engine::new(current_dir.clone())?;
            engine.sync().await?;
        }
        Commands::Snapshot { subcommand } => {
            let workspace_root = forge_core::find_forge_toml(&current_dir)
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| current_dir.clone());
            let cache_dir = forge_core::get_cache_dir()?;
            let manager = forge_core::SnapshotManager::new(&workspace_root, &cache_dir);

            match subcommand {
                SnapshotCommands::Create { name, description } => {
                    // Check forge.toml exists
                    let toml_path = workspace_root.join("forge.toml");
                    if !toml_path.exists() {
                        return Err("forge.toml not found. Run 'forge init' first.".to_string());
                    }

                    let snapshot_name = manager
                        .create(name.as_deref(), description.as_deref())
                        .map_err(|e| format!("Failed to create snapshot: {}", e))?;

                    let snap_dir = workspace_root
                        .join(".forge")
                        .join("snapshots")
                        .join(&snapshot_name);
                    println!("Snapshot saved: {} ({})", snapshot_name, snap_dir.display());
                }
                SnapshotCommands::List => {
                    let snapshots = manager.list()?;
                    if snapshots.is_empty() {
                        println!("No snapshots found");
                    } else {
                        println!(
                            "{:<20} | {:<25} | {:<14} | {}",
                            "Name", "Created At", "Runtime Count", "Description"
                        );
                        println!("{}", "-".repeat(80));
                        for snap in &snapshots {
                            let desc = snap
                                .description
                                .as_deref()
                                .unwrap_or("-");
                            println!(
                                "{:<20} | {:<25} | {:<14} | {}",
                                snap.name, snap.created_at, snap.runtime_count, desc
                            );
                        }
                    }
                }
                SnapshotCommands::SnapshotRestore { name, dry_run } => {
                    // File operations (backup + copy) — synchronous
                    manager
                        .restore(&name, dry_run)
                        .map_err(|e| format!("Failed to restore snapshot: {}", e))?;

                    if !dry_run {
                        // Run forge up: lock + sync
                        println!("Snapshot '{}' restored. Syncing runtimes...", name);
                        match run_operation(LockOperation, &current_dir, &event_bus).await {
                            Ok(_) => {
                                let engine = forge_core::Engine::new(current_dir.clone())?;
                                engine.sync().await?;
                                println!("Snapshot '{}' restored successfully.", name);
                            }
                            Err(e) => {
                                // Restore .bak files on failure
                                let bak_toml = workspace_root.join("forge.toml.bak");
                                let bak_lock = workspace_root.join("forge.lock.bak");
                                if bak_toml.exists() {
                                    let _ = std::fs::copy(
                                        &bak_toml,
                                        workspace_root.join("forge.toml"),
                                    );
                                }
                                if bak_lock.exists() {
                                    let _ = std::fs::copy(
                                        &bak_lock,
                                        workspace_root.join("forge.lock"),
                                    );
                                }
                                return Err(format!(
                                    "Snapshot restore failed, files reverted: {}",
                                    e
                                ));
                            }
                        }
                    }
                }
            }
        }
        Commands::PluginCommand(args) => {
            if args.is_empty() {
                return Err("Missing plugin command name".to_string());
            }
            let cmd_name = &args[0];
            let cmd_args: Vec<String> = args[1..].to_vec();

            // Look up plugin command by name (built-in already checked by clap)
            if let Some(cmd) = plugin_cli_commands.iter().find(|c| c.name() == cmd_name) {
                cmd.execute(&cmd_args)?;
            } else {
                // Check if there's a known plugin that didn't register in time
                let known: Vec<&str> = plugin_cmd_names.iter().map(|s| s.as_str()).collect();
                if known.is_empty() {
                    return Err(format!("Unknown command: '{}'. Run 'forge --help' for available commands.", cmd_name));
                } else {
                    return Err(format!(
                        "Unknown command: '{}'. Available plugin commands: {}",
                        cmd_name,
                        known.join(", ")
                    ));
                }
            }
        }
        Commands::Ai { subcommand } => {
            match subcommand {
                AiCommands::Context => {
                    let toml_path = forge_core::find_forge_toml(&current_dir);
                    let mut active_runtimes = HashMap::new();
                    if let Some(ref path) = toml_path {
                        if let Ok(config) = forge_core::load_config(path) {
                            active_runtimes = config.runtimes;
                        }
                    }
                    
                    let env_vars = if let Some(env_path) = forge_core::find_forge_env(&current_dir) {
                        let parsed = forge_core::parse_env_file(&env_path)?;
                        forge_core::mask_env_vars(&parsed)
                    } else {
                        HashMap::new()
                    };
                    
                    let context_json = json!({
                        "project_type": "rust_workspace",
                        "active_runtimes": active_runtimes,
                        "env_vars": env_vars
                    });
                    
                    println!("{}", serde_json::to_string_pretty(&context_json).unwrap());
                }
                AiCommands::Doctor => {
                    let cache_dir = forge_core::get_cache_dir()?;
                    let diag_ctx = forge_core::DiagnosticContext {
                        workspace_root: current_dir.clone(),
                        cache_dir,
                        mode: forge_core::DiagnosticMode::Deep,
                        active_profile: None,
                    };
                    let report = run_diagnostic_engine(&diag_ctx, &plugin_health_checks);
                    // Serialize with automatic masking (custom Serialize on Finding/QuickFixAction)
                    println!("{}", serde_json::to_string_pretty(&report).unwrap());
                }
            }
        }
    }
    Ok(())
}

// ── Policy helpers ─────────────────────────────────────────────────────

/// Build a [`PolicyEngine`] from `forge.toml` if a `[policy]` section exists.
/// Returns `None` when no `forge.toml` is found or the section is absent.
fn build_policy_engine(current_dir: &Path) -> Option<PolicyEngine> {
    let toml_path = forge_core::find_forge_toml(current_dir)?;
    let config = forge_core::load_config(&toml_path).ok()?;
    config.policy.as_ref().map(|pc| PolicyEngine::new(pc))
}

/// Check whether a lockfile exists and whether its entries include hashes.
fn compute_lockfile_state(current_dir: &Path) -> (bool, bool) {
    let workspace_root = forge_core::find_forge_toml(current_dir)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| current_dir.to_path_buf());
    let lockfile_path = workspace_root.join("forge.lock");
    if !lockfile_path.exists() {
        return (false, false);
    }
    match forge_core::load_lockfile(&lockfile_path) {
        Ok(lockfile) => {
            let has_hashes = lockfile.runtimes.iter().any(|r| !r.sha256.is_empty());
            (true, has_hashes)
        }
        Err(_) => (true, false),
    }
}

/// Run a fast diagnostic pass and return the health score (0–100).
fn compute_health_score(
    current_dir: &Path,
    plugin_health_checks: &[std::sync::Arc<dyn forge_core::HealthCheck>],
) -> u8 {
    match forge_core::get_cache_dir() {
        Ok(cache_dir) => {
            let diag_ctx = forge_core::DiagnosticContext {
                workspace_root: current_dir.to_path_buf(),
                cache_dir,
                mode: forge_core::DiagnosticMode::Fast,
                active_profile: None,
            };
            let report = run_diagnostic_engine(&diag_ctx, plugin_health_checks);
            report.health_score
        }
        Err(_) => 100,
    }
}

/// Get the active profile name (from `FORGE_PROFILE` env var or first profile key).
fn get_active_profile(current_dir: &Path) -> Option<String> {
    std::env::var("FORGE_PROFILE").ok().or_else(|| {
        let toml_path = forge_core::find_forge_toml(current_dir)?;
        forge_core::load_config(&toml_path).ok().and_then(|c| {
            c.profile.and_then(|p| p.keys().next().cloned())
        })
    })
}

/// Get the list of active runtime names from `forge.toml`.
fn get_active_runtimes(current_dir: &Path) -> Vec<String> {
    let toml_path = match forge_core::find_forge_toml(current_dir) {
        Some(p) => p,
        None => return Vec::new(),
    };
    match forge_core::load_config(&toml_path) {
        Ok(config) => config.runtimes.into_keys().collect(),
        Err(_) => Vec::new(),
    }
}

/// Print policy violations to stderr and abort the process.
fn enforce_policy(violations: Vec<forge_core::PolicyViolation>) {
    if violations.is_empty() {
        return;
    }
    for v in &violations {
        eprintln!("{}", v);
    }
    std::process::exit(1);
}

fn get_shim_binary_path() -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let bin_dir = current_exe.parent()
        .ok_or_else(|| "Failed to get parent directory of current executable".to_string())?;
        
    let name = if cfg!(windows) { "forge-shim.exe" } else { "forge-shim" };
    let candidate = bin_dir.join(name);
    if candidate.exists() {
        return Ok(candidate);
    }
    
    let target_debug = bin_dir.parent().map(|p| p.join(name)).unwrap_or_default();
    if target_debug.exists() {
        return Ok(target_debug);
    }
    
    Err(format!(
        "Could not find forge-shim executable. Looked in {:?} and {:?}",
        candidate, target_debug
    ))
}

fn get_home_dir() -> Option<PathBuf> {
    if let Some(forge_home) = std::env::var_os("FORGE_HOME") {
        Some(PathBuf::from(forge_home))
    } else {
        dirs::home_dir()
    }
}

fn setup_shims() -> Result<(), String> {
    let shim_src = get_shim_binary_path()?;
    let home = get_home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let dest_dir = home.join(".forge").join("bin");
    
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create directory {:?}: {}", dest_dir, e))?;
        
    let runtimes = ["node", "python", "bun", "go", "cargo", "rust"];
    for rt in &runtimes {
        let dest_name = if cfg!(windows) {
            format!("{}.exe", rt)
        } else {
            rt.to_string()
        };
        let dest_path = dest_dir.join(dest_name);
        
        std::fs::copy(&shim_src, &dest_path)
            .map_err(|e| format!("Failed to copy shim to {:?}: {}", dest_path, e))?;
            
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&dest_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&dest_path, perms);
            }
        }
        println!("Created shim: {:?}", dest_path);
    }
    
    println!("Forge shims setup complete. Placed in {:?}", dest_dir);
    Ok(())
}

fn uninstall_shims() -> Result<(), String> {
    let home = get_home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let dest_dir = home.join(".forge").join("bin");
    
    let runtimes = ["node", "python", "bun", "go", "cargo", "rust"];
    for rt in &runtimes {
        let dest_name = if cfg!(windows) {
            format!("{}.exe", rt)
        } else {
            rt.to_string()
        };
        let dest_path = dest_dir.join(dest_name);
        if dest_path.exists() {
            std::fs::remove_file(&dest_path)
                .map_err(|e| format!("Failed to remove shim {:?}: {}", dest_path, e))?;
            println!("Removed shim: {:?}", dest_path);
        }
    }
    
    if dest_dir.exists() {
        let _ = std::fs::remove_dir(&dest_dir);
    }
    
    println!("Forge shims uninstalled successfully.");
    Ok(())
}

fn run_diagnostic_engine(
    ctx: &forge_core::DiagnosticContext,
    plugin_health_checks: &[std::sync::Arc<dyn forge_core::HealthCheck>],
) -> forge_core::DiagnosticReport {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    let mut engine = forge_core::DiagnosticEngine::new();
                    engine.register_plugin_checks(plugin_health_checks.to_vec());
                    engine.run(ctx).await
                })
            })
        }
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime for diagnostics");
            rt.block_on(async {
                let mut engine = forge_core::DiagnosticEngine::new();
                engine.register_plugin_checks(plugin_health_checks.to_vec());
                engine.run(ctx).await
            })
        }
    }
}

fn render_finding_table(report: &forge_core::DiagnosticReport) {
    let score = report.health_score;
    let score_label = if score >= 90 {
        "Healthy"
    } else if score >= 70 {
        "Needs Attention"
    } else if score >= 40 {
        "Degraded"
    } else {
        "Critical"
    };
    println!("\n  Health Score: {}/100 ({})", score, score_label);
    println!("  Mode: {:?} | Elapsed: {}ms\n", report.mode, report.elapsed_ms);

    if report.findings.is_empty() {
        println!("  No issues found. Environment is healthy.");
        return;
    }

    println!("  {:<8} {:<10} {:<6} {}", "CODE", "SEVERITY", "CONF", "MESSAGE");
    println!("  {}", "-".repeat(72));
    for f in &report.findings {
        let sev = match f.severity {
            forge_core::Severity::INFO => "INFO",
            forge_core::Severity::WARNING => "WARNING",
            forge_core::Severity::ERROR => "ERROR",
            forge_core::Severity::CRITICAL => "CRITICAL",
        };
        println!("  {:<8} {:<10} {:>3}%  {}", f.code, sev, f.confidence, f.message);
    }
    println!();

    // Show quick fixes
    let fixable: Vec<_> = report.findings.iter()
        .filter(|f| f.suggested_quick_fix.is_some())
        .collect();
    if !fixable.is_empty() {
        println!("  Suggested Fixes:");
        for f in fixable {
            if let Some(ref qf) = f.suggested_quick_fix {
                println!("    {} -> {}", f.code, qf.description);
            }
        }
        println!();
    }
}

fn run_doctor(current_dir: &Path, deep: bool, json: bool, plugin_health_checks: &[std::sync::Arc<dyn forge_core::HealthCheck>]) -> Result<(), String> {
    let cache_dir = forge_core::get_cache_dir()?;
    let mode = if deep {
        forge_core::DiagnosticMode::Deep
    } else {
        forge_core::DiagnosticMode::Fast
    };
    let diag_ctx = forge_core::DiagnosticContext {
        workspace_root: current_dir.to_path_buf(),
        cache_dir,
        mode,
        active_profile: None,
    };
    let report = run_diagnostic_engine(&diag_ctx, plugin_health_checks);

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        render_finding_table(&report);
    }
    Ok(())
}

fn run_which(current_dir: &Path, runtime: &str) -> Result<(), String> {
    let toml_path = forge_core::find_forge_toml(current_dir);
    let active_project = toml_path.as_ref().map(|p| p.parent().unwrap().to_string_lossy().into_owned()).unwrap_or_else(|| "none".to_string());
    
    let mut resolved_path = None;
    let mut source = "none".to_string();
    let mut version = "unknown".to_string();
    
    // 1. Try to check local cache
    if let Some(ref path) = toml_path {
        let cache_file = path.parent().unwrap().join(".forge").join("shims.cache");
        if cache_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&cache_file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some(pos) = trimmed.find('=') {
                        let key = trimmed[..pos].trim();
                        if key.eq_ignore_ascii_case(runtime) {
                            let value = trimmed[pos + 1..].trim();
                            let path_buf = std::path::PathBuf::from(value);
                            if path_buf.exists() {
                                resolved_path = Some(path_buf);
                                source = "local".to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        // If local, find version in lockfile
        if source == "local" {
            let lock_file_path = path.parent().unwrap().join("forge.lock");
            if let Ok(lockfile) = forge_core::load_lockfile(&lock_file_path) {
                if let Some(rt) = lockfile.runtimes.iter().find(|r| r.name.eq_ignore_ascii_case(runtime)) {
                    version = rt.version.clone();
                }
            }
        }
    }
    
    // 2. If not local, check global PATH (filtering out ~/.forge/bin)
    if resolved_path.is_none() {
        let home = get_home_dir().unwrap_or_default();
        let shim_dir = home.join(".forge").join("bin");
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
            .filter(|p| {
                if let (Ok(p_canon), Ok(shim_canon)) = (p.canonicalize(), shim_dir.canonicalize()) {
                    p_canon != shim_canon
                } else {
                    p != &shim_dir
                }
            })
            .collect();
        
        for dir in filtered_paths {
            #[cfg(windows)]
            {
                let extensions = ["exe", "cmd", "bat", "ps1"];
                for ext in &extensions {
                    let candidate = dir.join(format!("{}.{}", runtime, ext));
                    if candidate.is_file() {
                        resolved_path = Some(candidate);
                        source = "global".to_string();
                        version = "system".to_string();
                        break;
                    }
                }
            }
            #[cfg(not(windows))]
            {
                let candidate = dir.join(runtime);
                if candidate.is_file() {
                    resolved_path = Some(candidate);
                    source = "global".to_string();
                    version = "system".to_string();
                    break;
                }
            }
            if resolved_path.is_some() {
                break;
            }
        }
    }
    
    match resolved_path {
        Some(path) => {
            println!("Runtime: {}", runtime);
            println!("Path: {}", path.display());
            println!("Source: {}", source);
            println!("Version: {}", version);
            println!("Active Project: {}", active_project);
            Ok(())
        }
        None => {
            println!("Runtime '{}' not resolved.", runtime);
            Err(format!("Runtime '{}' is not available globally or locally.", runtime))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_shim_args_and_exit_code_propagation() {
        let temp_dir = std::env::temp_dir().join("forge_cli_integration_test");
        fs::create_dir_all(&temp_dir).unwrap();

        let mock_target = if cfg!(windows) {
            temp_dir.join("mock_target.bat")
        } else {
            temp_dir.join("mock_target.sh")
        };

        if cfg!(windows) {
            fs::write(&mock_target, "@echo off\necho target_ran %*\nexit /b 42\n").unwrap();
        } else {
            fs::write(&mock_target, "#!/bin/sh\necho \"target_ran $@\"\nexit 42\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&mock_target).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&mock_target, perms).unwrap();
            }
        }

        let forge_dir = temp_dir.join(".forge");
        fs::create_dir_all(&forge_dir).unwrap();
        let cache_file = forge_dir.join("shims.cache");
        fs::write(
            &cache_file,
            format!("# forge-shims-cache-v1\nmytool = {}\n", mock_target.to_str().unwrap().replace('\\', "/")),
        )
        .unwrap();

        fs::write(temp_dir.join("forge.toml"), "[runtimes]\n").unwrap();

        let current_exe = std::env::current_exe().unwrap();
        let build_dir = current_exe.parent().unwrap();
        let shim_exe_name = if cfg!(windows) { "forge-shim.exe" } else { "forge-shim" };
        let shim_src = build_dir.join(shim_exe_name);
        
        let mut shim_path = shim_src.clone();
        if !shim_path.exists() {
            if let Some(parent) = build_dir.parent() {
                let alt = parent.join(shim_exe_name);
                if alt.exists() {
                    shim_path = alt;
                }
            }
        }
        
        if !shim_path.exists() {
            let alt_deps = build_dir.join("deps").join(shim_exe_name);
            if alt_deps.exists() {
                shim_path = alt_deps;
            }
        }

        if !shim_path.exists() {
            panic!("Could not find forge-shim compiled executable at {:?}", shim_src);
        }

        let shim_copy_name = if cfg!(windows) { "mytool.exe" } else { "mytool" };
        let shim_copy = temp_dir.join(shim_copy_name);
        fs::copy(&shim_path, &shim_copy).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&shim_copy).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&shim_copy, perms).unwrap();
        }

        let output = Command::new(&shim_copy)
            .args(&["hello", "world"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        println!("STDOUT: {}", stdout);
        println!("STDERR: {}", stderr);

        assert_eq!(output.status.code(), Some(42));
        assert!(stdout.contains("target_ran"));
        assert!(stdout.contains("hello"));
        assert!(stdout.contains("world"));

        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_setup_and_uninstall_shims() {
        let temp_dir = std::env::temp_dir().join("forge_cli_setup_uninstall_test");
        fs::create_dir_all(&temp_dir).unwrap();

        // Override FORGE_HOME env var to use temp_dir
        let old_forge_home = std::env::var("FORGE_HOME").ok();
        std::env::set_var("FORGE_HOME", &temp_dir);

        // Run setup
        let res_setup = setup_shims();
        assert!(res_setup.is_ok(), "setup_shims failed: {:?}", res_setup);

        let bin_dir = temp_dir.join(".forge").join("bin");
        assert!(bin_dir.exists());

        let runtimes = ["node", "python", "bun", "go", "cargo", "rust"];
        for rt in &runtimes {
            let dest_name = if cfg!(windows) {
                format!("{}.exe", rt)
            } else {
                rt.to_string()
            };
            assert!(bin_dir.join(dest_name).exists());
        }

        // Run uninstall
        let res_uninstall = uninstall_shims();
        assert!(res_uninstall.is_ok(), "uninstall_shims failed: {:?}", res_uninstall);

        for rt in &runtimes {
            let dest_name = if cfg!(windows) {
                format!("{}.exe", rt)
            } else {
                rt.to_string()
            };
            assert!(!bin_dir.join(dest_name).exists());
        }

        // The directory should also be removed if empty
        assert!(!bin_dir.exists());

        // Restore env vars
        if let Some(val) = old_forge_home {
            std::env::set_var("FORGE_HOME", val);
        } else {
            std::env::remove_var("FORGE_HOME");
        }

        fs::remove_dir_all(&temp_dir).ok();
    }

    /// 4.5: Plugin CliCommand dispatch via PluginRegistry.
    #[test]
    fn test_plugin_cli_command_dispatch() {
        use forge_core::{CliCommand, ExtensionSink, Plugin, PluginRegistry};

        struct TestPlugin;

        impl Plugin for TestPlugin {
            fn name(&self) -> &str {
                "test-cli-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String> {
                sink.add_cli_command(Box::new(TestCmd));
                Ok(())
            }
        }

        #[derive(Clone)]
        struct TestCmd;

        impl CliCommand for TestCmd {
            fn name(&self) -> &str {
                "mycmd"
            }
            fn description(&self) -> &str {
                "A test plugin command"
            }
            fn execute(&self, args: &[String]) -> Result<(), String> {
                if args.contains(&"--flag".to_string()) {
                    Ok(())
                } else {
                    Err("expected --flag".to_string())
                }
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin)).unwrap();
        registry.resolve_and_init().unwrap();

        // Query plugin commands and verify dispatch
        let cmds: Vec<Box<dyn CliCommand>> = registry.drain_cli_commands();
        let cmd = cmds.iter().find(|c| c.name() == "mycmd").unwrap();
        assert_eq!(cmd.name(), "mycmd");
        // Dispatch with args
        assert!(cmd.execute(&["--flag".to_string()]).is_ok());
        // Dispatch without expected arg should fail
        assert!(cmd.execute(&[]).is_err());
    }

    /// Verify built-in command name takes precedence over plugin commands.
    /// The CLI now explicitly rejects conflicting plugin command names at startup.
    #[test]
    fn test_plugin_builtin_precedence() {
        use forge_core::{CliCommand, ExtensionSink, Plugin, PluginRegistry};

        // Plugin registering a command that conflicts with built-in "init"
        struct ConflictingPlugin;

        impl Plugin for ConflictingPlugin {
            fn name(&self) -> &str {
                "conflict-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String> {
                sink.add_cli_command(Box::new(ConflictingCmd));
                Ok(())
            }
        }

        struct ConflictingCmd;

        impl CliCommand for ConflictingCmd {
            fn name(&self) -> &str {
                "init"
            }
            fn description(&self) -> &str {
                "Conflicting command"
            }
            fn execute(&self, _args: &[String]) -> Result<(), String> {
                Ok(())
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(ConflictingPlugin)).unwrap();
        registry.resolve_and_init().unwrap();

        // The plugin command named "init" conflicts with the built-in "init".
        // drain_cli_commands still returns it — the name conflict check
        // happens at CLI startup in run_cli(), where builtin_command_names
        // is checked and a warning is emitted.
        let cmds: Vec<Box<dyn CliCommand>> = registry.drain_cli_commands();
        let conflicting = cmds.iter().find(|c| c.name() == "init");
        assert!(conflicting.is_some(), "Plugin command should still be in registry");
        assert_eq!(conflicting.unwrap().name(), "init");

        // Simulate the CLI startup check: commands matching builtins get rejected
        let filtered: Vec<&Box<dyn CliCommand>> = cmds.iter()
            .filter(|c| !BUILTIN_COMMANDS.contains(&c.name()))
            .collect();
        assert!(filtered.is_empty(), "Conflicting commands should be filtered out");
    }

    // ── Phase 5: Explain tests ──────────────────────────────────────────

    /// 5.6: Backward compat — `forge explain node` parses as Runtime { name: "node" }
    #[test]
    fn test_explain_parse_runtime_backward_compat() {
        let cmd = ExplainSubcommand::parse(&["node".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Runtime { name } if name == "node"));
        let cmd = ExplainSubcommand::parse(&["python".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Runtime { name } if name == "python"));
    }

    /// 5.7a: Subcommand `operation` parses correctly
    #[test]
    fn test_explain_parse_operation() {
        let cmd = ExplainSubcommand::parse(&["operation".to_string(), "abc-123".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Operation { id } if id == "abc-123"));
        // Short alias
        let cmd = ExplainSubcommand::parse(&["op".to_string(), "xyz".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Operation { id } if id == "xyz"));
    }

    /// 5.2: Operation with missing ID returns error
    #[test]
    fn test_explain_parse_operation_missing_id() {
        let err = ExplainSubcommand::parse(&["operation".to_string()]).unwrap_err();
        assert!(err.contains("Usage: forge explain operation"));
    }

    /// 5.7b: Subcommand `context` parses correctly
    #[test]
    fn test_explain_parse_context() {
        let cmd = ExplainSubcommand::parse(&["context".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Context));
        let cmd = ExplainSubcommand::parse(&["ctx".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Context));
    }

    /// 5.7c: Subcommand `config` parses correctly
    #[test]
    fn test_explain_parse_config() {
        let cmd = ExplainSubcommand::parse(&["config".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Config));
        let cmd = ExplainSubcommand::parse(&["cfg".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Config));
    }

    /// 5.7d: Subcommand `profile` parses correctly
    #[test]
    fn test_explain_parse_profile() {
        let cmd = ExplainSubcommand::parse(&["profile".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Profile));
        let cmd = ExplainSubcommand::parse(&["prof".to_string()]).unwrap();
        assert!(matches!(cmd, ExplainSubcommand::Profile));
    }

    /// 5.7e: Empty args returns error
    #[test]
    fn test_explain_parse_empty() {
        let err = ExplainSubcommand::parse(&[] as &[String]).unwrap_err();
        assert!(err.contains("Missing"));
    }

    /// 5.1: print_operation_table produces formatted output without panicking
    #[test]
    fn test_print_operation_table() {
        let summary = OperationSummary {
            id: "test-op-1".to_string(),
            runtime: "node".to_string(),
            duration_ms: 1234,
            status: "Success".to_string(),
        };
        let trace = "├── Resolve (500ms)\n│   └── Download (300ms)\n└── Extract (200ms)\n".to_string();
        // Verify it doesn't panic with valid input
        print_operation_table(&summary, &trace);
    }

    /// 5.4: print_config_table handles vars with sources and secret masking
    #[test]
    fn test_print_config_table() {
        use forge_core::manifest::ConfigSection;
        use forge_core::secrets::VarMetadata;

        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("API_KEY".to_string(), "secret-value".to_string());
        vars.insert("PORT".to_string(), "3000".to_string());

        let mut metadata: HashMap<String, VarMetadata> = HashMap::new();
        metadata.insert(
            "API_KEY".to_string(),
            VarMetadata { key: "API_KEY".to_string(), source: ValueSource::SystemEnv },
        );
        metadata.insert(
            "PORT".to_string(),
            VarMetadata { key: "PORT".to_string(), source: ValueSource::EnvFile },
        );

        let resolved = ResolvedEnvironment { vars, metadata };

        let mut definitions = HashMap::new();
        definitions.insert(
            "API_KEY".to_string(),
            forge_core::manifest::ConfigDefinition {
                val_type: Some("string".to_string()),
                required: true,
                default: None,
                pattern: None,
                description: Some("API key".to_string()),
                secret: true,
            },
        );
        let config_section = ConfigSection { definitions };
        print_config_table(&resolved, Some(&config_section));
    }

    /// 5.5: print_profile_table renders profile output without panicking
    #[test]
    fn test_print_profile_table() {
        use forge_core::secrets::VarMetadata;

        // Create profile section without needing toml::Value directly

        let config = ForgeConfig {
            runtimes: HashMap::new(),
            workspace_id: None,
            config: None,
            profile: None, // profile is None for this smoke test
            policy: None,
        };

        let mut vars: HashMap<String, String> = HashMap::new();
        vars.insert("NODE_ENV".to_string(), "development".to_string());

        let mut metadata: HashMap<String, VarMetadata> = HashMap::new();
        metadata.insert(
            "NODE_ENV".to_string(),
            VarMetadata { key: "NODE_ENV".to_string(), source: ValueSource::ProfileOverlay("dev".to_string()) },
        );

        let resolved = ResolvedEnvironment { vars, metadata };
        // Call with None profile so it prints "No profile variables defined" path
        print_profile_table("dev", &config, &resolved);
    }

    /// 5.3: print_context_table renders provider results without panicking
    #[test]
    fn test_print_context_table() {
        use serde_json::json;
        let ctx = ForgeContext {
            schema_version: "1.0.0".to_string(),
            runtimes: json!({"node": "18.0.0"}),
            config: json!({"forge.toml": "/test/forge.toml"}),
            diagnostics: json!({"health": "ok"}),
            workspace: json!({"total_files": 500, "max_depth": 5}),
            environment: json!({"PATH": "/usr/bin"}),
            secrets_metadata: json!({"MY_SECRET": {"source": "keyring"}}),
        };
        print_context_table(&ctx);
    }
}
