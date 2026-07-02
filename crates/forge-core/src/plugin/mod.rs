// TODO: tests — registry.rs has unit tests; plugin lifecycle needs e2e
pub mod registry;

pub use registry::PluginRegistry;

use crate::resolver::RuntimeProvider;
use crate::context::{ContextProvider, ContextExporter};
use crate::diagnostics::HealthCheck;
use crate::secrets::ConfigurationProvider;

/// Current API version for the plugin system.
/// Plugins MUST match this version to be accepted.
pub const FORGE_PLUGIN_API_VERSION: &str = "1.0.0";

/// All plugins MUST implement the `Plugin` trait.
///
/// The lifecycle is:
/// 1. `PluginRegistry::register()` stores the plugin (pending init)
/// 2. `PluginRegistry::resolve_and_init()` resolves dependency DAG,
///    then calls each plugin's `register()` in topological order
/// 3. During `register()`, the plugin injects its extension types
///    via the `ExtensionSink` interface
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    /// API version of this plugin. Defaults to `FORGE_PLUGIN_API_VERSION`.
    fn api_version(&self) -> &str {
        FORGE_PLUGIN_API_VERSION
    }

    /// Other plugins this plugin depends on (by name).
    fn depends_on(&self) -> &[&str] {
        &[]
    }

    /// Called during `resolve_and_init()` after dependency resolution.
    /// Use `sink` to register extension types with the engine.
    fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String>;
}

/// A plugin-registered CLI subcommand.
///
/// Plugins implementing this trait can expose custom CLI commands
/// via `forge <plugin-command> [...args]`.
pub trait CliCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> Result<(), String>;
}

/// Errors that can occur during plugin registration and initialization.
#[derive(Debug)]
pub enum PluginError {
    /// A plugin with this name is already registered.
    DuplicateName(String),
    /// The plugin's `api_version` does not match `FORGE_PLUGIN_API_VERSION`.
    ApiVersionMismatch {
        plugin: String,
        expected: String,
        actual: String,
    },
    /// A cycle was detected in the dependency graph.
    CyclicDependency(Vec<String>),
    /// The plugin's `register()` returned an error.
    RegistrationFailed(String, String),
    /// The plugin's `register()` panicked.
    PanicInRegister {
        name: String,
        message: Option<String>,
    },
    /// `scan_directory()` is not yet implemented (stub).
    ScanNotImplemented,
    /// `register()` was called after `resolve_and_init()` completed.
    AlreadyInitialized(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::DuplicateName(name) => {
                write!(f, "Duplicate plugin: '{}' is already registered", name)
            }
            PluginError::ApiVersionMismatch { plugin, expected, actual } => {
                write!(
                    f,
                    "API version mismatch for plugin '{}': expected {}, got {}",
                    plugin, expected, actual
                )
            }
            PluginError::CyclicDependency(plugins) => {
                write!(f, "Cyclic dependency detected: {}", plugins.join(" → "))
            }
            PluginError::RegistrationFailed(name, reason) => {
                write!(f, "Plugin '{}' registration failed: {}", name, reason)
            }
            PluginError::PanicInRegister { name, message } => {
                write!(f, "Plugin '{}' panicked during register()", name)?;
                if let Some(msg) = message {
                    write!(f, ": {}", msg)?;
                }
                Ok(())
            }
            PluginError::ScanNotImplemented => {
                write!(f, "scan_directory() is not yet implemented")
            }
            PluginError::AlreadyInitialized(name) => {
                write!(f, "Plugin '{}' was registered after initialization completed", name)
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// Interface through which plugins register their extension types
/// during `Plugin::register()`.
///
/// The host engine uses this to collect runtime providers, context
/// providers/exporters, health checks, configuration providers,
/// operations, and CLI commands from each plugin.
pub trait ExtensionSink {
    fn add_runtime_provider(&mut self, provider: Box<dyn RuntimeProvider>);
    fn add_context_provider(&mut self, provider: std::sync::Arc<dyn ContextProvider>);
    fn add_context_exporter(&mut self, exporter: std::sync::Arc<dyn ContextExporter>);
    fn add_health_check(&mut self, check: std::sync::Arc<dyn HealthCheck>);
    fn add_configuration_provider(&mut self, provider: Box<dyn ConfigurationProvider>);
    fn add_cli_command(&mut self, cmd: Box<dyn CliCommand>);
    fn add_operation(&mut self, op: Box<dyn crate::operations::Operation>);
}
