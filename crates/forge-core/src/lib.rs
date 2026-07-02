use std::path::Path;

pub mod types;
pub mod manifest;
pub mod registry;
pub mod resolver;
pub mod installer;
pub mod cache;
pub mod environment;
pub mod launcher;
pub mod lock;
pub mod event_bus;
pub mod operations;
pub mod api;
pub mod secrets;
pub mod diagnostics;
pub mod context;
pub mod plugin;
pub mod state;
pub mod bundle;
pub mod snapshot;

pub use diagnostics::{
    DiagnosticContext, DiagnosticMode, Severity, Explanation, QuickFix, QuickFixAction,
    Finding, DiagnosticReport, HealthCheck, DiagnosticEngine, calculate_health_score,
};
pub use context::{
    ContextProvider, ContextExporter, AgentAdapter, ContextEngine, ContextOptions,
    ForgeContext, FcpHandshake, FcpHandshakeRequest, FcpHandshakeResponse,
    RuntimeProviderImpl, ConfigurationProviderImpl, DiagnosticsProviderImpl,
    WorkspaceProviderImpl, EnvironmentProviderImpl, SecretsProviderImpl,
    JsonExporter, MarkdownExporter, McpExporter,
    ClaudeCodeAdapter, GeminiCliAdapter, AiderAdapter, ContinueAdapter,
};

// Re-export API types
pub use api::v1::{Engine, RuntimeExplanation, OperationSummary};
pub use secrets::{
    SecretProvider, ConfigurationProvider, ValueSource, VarMetadata, ResolvedEnvironment,
    KeyringSecretProvider, MockSecretProvider, FallbackSecretProvider, EncryptedPayload,
};



// Re-export stable types from types.rs
pub use types::{
    RuntimeId, RuntimeVersion, Hash, Platform, Architecture, RuntimeLock, EmulationLog, Lockfile,
    LifecycleState, OperationResult, ChangeRecord, Event, EventStatus, OperationStatus,
};

// Re-export event_bus
pub use event_bus::EventBus;

// Re-export plugin types
pub use plugin::{
    Plugin, CliCommand, PluginRegistry, PluginError, ExtensionSink, FORGE_PLUGIN_API_VERSION,
};

// Re-export operations types
pub use operations::{Plan, Operation, Context, SimplePlan, SyncPlan, RepairPlan};

// Re-export stable types/functions from manifest.rs
pub use manifest::{ForgeConfig, find_forge_toml, load_config};

// Re-export stable types/functions from registry.rs
pub use registry::{
    HybridRegistry, RegistryEntry, FrrsMetadata, FrrsArtifact, RegistryIndex, RegistryIndexEntry,
    RemoteRegistry,
    normalize_arch, normalize_platform, detect_platform, detect_arch,
};

// Re-export stable types/functions from resolver.rs
pub use resolver::{
    RuntimeProvider, Resolver, resolve_from_registry, resolve_runtime_lock,
    NodeProvider, PythonProvider, BunProvider, GoProvider, RustProvider,
    LlvmProvider, JdkProvider,
    DoctorIssue, resolve_environment, validate_environment, interpolate_value,
};

// Re-export stable types/functions from installer.rs
pub use installer::{
    Extractor, ZipExtractor, TarGzExtractor, TarXzExtractor, check_path_traversal,
    extract_archive, download_runtime, install_runtimes,
};

// Re-export stable types/functions from cache.rs
pub use cache::{
    get_cache_dir, find_bin_dirs, generate_shims_cache_map, write_shims_cache_file,
    regenerate_shims_cache, append_to_gitignore,
};

// Re-export stable types/functions from environment.rs
pub use environment::{find_forge_env, parse_env_file, is_secret, mask_env_vars, RuntimeContextProvider, materialize_environment};

// Re-export stable types/functions from launcher.rs
pub use launcher::{run_command_in_env, spawn_shell_in_env};

// Re-export stable types/functions from lock.rs
pub use lock::{load_lockfile, save_lockfile};

// Re-export state management functions
pub use state::{compute_current_state, save_state};

// Re-export bundle types and functions
pub use bundle::{
    Bundle, BundleMetadata, BundleError, BundleChecksums, ChecksumEntry,
    create_bundle, restore_bundle, verify_checksums,
};

// Re-export snapshot types and functions
pub use snapshot::{SnapshotManager, SnapshotMetadata};

/// Orchestrates updating the lockfile from forge.toml config.
///
/// The resolution chain:
///   1. Flat entries loaded from `metadata_cache.toml` (legacy)
///   2. FRRS cache directory (`.forge/metadata_cache/`)
///   3. Remote FRRS registry (HTTP, best-effort pre-fetch)
///   4. Embedded compiled-in defaults
///
/// The remote registry URL is configured via `FORGE_REGISTRY_URL` env var
/// (default: `https://registry.forge.sh`). Set to empty string to disable
/// remote fetching entirely (offline mode, REQ-REG-009).
pub async fn update_lockfile(toml_path: &Path, lockfile_path: &Path) -> Result<Lockfile, String> {
    let config = load_config(toml_path)?;
    let mut lockfile = load_lockfile(lockfile_path).unwrap_or_default();
    let platform = detect_platform();
    let arch = detect_arch();

    let workspace_dir = toml_path.parent().unwrap_or(Path::new("."));
    let registry_path = workspace_dir.join(".forge").join("metadata_cache.toml");
    let frrs_cache_dir = workspace_dir.join(".forge").join("metadata_cache");

    // Build base registry from legacy flat file or defaults
    let mut registry = if registry_path.exists() {
        HybridRegistry::load_from_file(&registry_path)?
    } else {
        HybridRegistry::default_with_internal()
    };

    // Attach FRRS cache directory
    registry = registry.with_cache_dir(frrs_cache_dir.clone());

    // Configure remote registry via FORGE_REGISTRY_URL env var
    let registry_url = std::env::var("FORGE_REGISTRY_URL")
        .unwrap_or_else(|_| "https://registry.forge.sh".to_string());

    if !registry_url.is_empty() {
        let remote = RemoteRegistry::new(&registry_url, frrs_cache_dir);
        registry = registry.with_remote(remote);
    }

    let resolver = Resolver::new();

    let mut new_runtimes = Vec::new();
    for (name, _version_req) in &config.runtimes {
        // Best-effort pre-fetch from remote to populate FRRS cache
        if registry.remote.is_some() {
            let _ = registry.refresh_remote(name).await;
        }

        let resolved = resolver.resolve(name, _version_req, platform, arch, &registry)?;
        new_runtimes.push(resolved);
    }

    lockfile.runtimes = new_runtimes;
    save_lockfile(lockfile_path, &lockfile)?;
    if let Ok(cache_dir) = get_cache_dir() {
        let _ = regenerate_shims_cache(&lockfile, &cache_dir, workspace_dir);
    }
    Ok(lockfile)
}
