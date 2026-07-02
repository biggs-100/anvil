use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::context::{ContextExporter, ContextProvider};
use crate::diagnostics::HealthCheck;
use crate::operations::Operation;
use crate::resolver::RuntimeProvider;
use crate::secrets::ConfigurationProvider;

use super::{CliCommand, ExtensionSink, Plugin, PluginError, FORGE_PLUGIN_API_VERSION};

/// Registry for all registered plugins and their extension types.
///
/// Lifecycle:
/// 1. `register()` – stores a plugin, validates name uniqueness.
/// 2. `resolve_and_init()` – resolves dependency DAG, calls each
///    plugin's `register()` in topological order, collects extensions.
/// 3. Query methods (`runtime_providers()`, `context_providers()`, etc.)
///    return the collected extensions for integration with the engine.
///
/// ## Design constraints
///
/// - `scan_directory()` is a stub that returns `Ok(())` (future dynamic loading).
/// - Error isolation: `catch_unwind` around each plugin's `register()`.
pub struct PluginRegistry {
    /// Plugins pending initialization.
    pending: Vec<Box<dyn Plugin>>,
    /// Whether `resolve_and_init()` has completed.
    initialized: bool,

    // ── Extension collections (populated during init) ──
    runtime_providers: Vec<Box<dyn RuntimeProvider>>,
    context_providers: Vec<Arc<dyn ContextProvider>>,
    context_exporters: Vec<Arc<dyn ContextExporter>>,
    health_checks: Vec<Arc<dyn HealthCheck>>,
    config_providers: Vec<Box<dyn ConfigurationProvider>>,
    cli_commands: Vec<Box<dyn CliCommand>>,
    operations: Vec<Box<dyn Operation>>,
}

impl PluginRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            initialized: false,
            runtime_providers: Vec::new(),
            context_providers: Vec::new(),
            context_exporters: Vec::new(),
            health_checks: Vec::new(),
            config_providers: Vec::new(),
            cli_commands: Vec::new(),
            operations: Vec::new(),
        }
    }

    /// Registers a plugin for subsequent initialization.
    ///
    /// Validates:
    /// - No duplicate names
    /// - API version match (REQ-PLUG-003)
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        let name = plugin.name().to_string();

        // Reject registration after initialization
        if self.initialized {
            return Err(PluginError::AlreadyInitialized(name));
        }

        let api_version = plugin.api_version().to_string();

        // Reject duplicate names
        if self.pending.iter().any(|p| p.name() == name) {
            return Err(PluginError::DuplicateName(name));
        }

        // Reject API version mismatch (task 2.1)
        if api_version != FORGE_PLUGIN_API_VERSION {
            return Err(PluginError::ApiVersionMismatch {
                plugin: name,
                expected: FORGE_PLUGIN_API_VERSION.to_string(),
                actual: api_version,
            });
        }

        self.pending.push(plugin);
        Ok(())
    }

    /// Scans a directory for dynamic plugin artifacts.
    ///
    /// **Stub** – currently returns `Ok(())`. Designed for future
    /// dynamic loading via `libloading` or similar.
    pub fn scan_directory(_path: &Path) -> Result<(), PluginError> {
        // Future: scan .so / .dll / .dylib artifacts
        Ok(())
    }

    /// Resolves the dependency DAG and initialises all pending plugins.
    ///
    /// Steps:
    /// 1. Topological sort with cycle detection (DFS).
    /// 2. For each plugin in order, call `register()` inside `catch_unwind`.
    /// 3. Collect all extension types into the registry.
    ///
    /// If a plugin panics or returns an error, the whole init aborts
    /// and the engine is left in an uninitialized state.
    ///
    /// **Note**: This method is NOT idempotent on partial success. If a
    /// plugin succeeds and a later one fails, the caller MUST NOT retry
    /// `resolve_and_init()` without re-registering every plugin, because
    /// `register()` for the already-successful plugins would run again
    /// and produce duplicate extensions. Bug 1 fix: the extension vectors
    /// ARE cleared before returning the error, so a subsequent retry
    /// starts from a clean state — but each plugin's `register()` will
    /// still be called again, so the caller must ensure the plugins'
    /// `register()` is idempotent or the caller re-registers everything
    /// from scratch.
    pub fn resolve_and_init(&mut self) -> Result<(), PluginError> {
        if self.initialized {
            return Ok(());
        }

        let plugins = std::mem::take(&mut self.pending);

        let names: Vec<String> = plugins.iter().map(|p| p.name().to_string()).collect();
        let name_to_idx: HashMap<&str, usize> =
            names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); plugins.len()];
        for (i, plugin) in plugins.iter().enumerate() {
            for dep in plugin.depends_on() {
                if let Some(&dep_idx) = name_to_idx.get(dep) {
                    adj[i].push(dep_idx);
                }
            }
        }
        let order = topological_sort(&names, &adj)?;

        for &idx in &order {
            let name = plugins[idx].name().to_string();

            // Build a RegistrationSink that borrows self's extension vecs.
            // This avoids aliasing between `plugins` (local vec) and `self`.
            let mut sink = RegistrationSink {
                runtime_providers: &mut self.runtime_providers,
                context_providers: &mut self.context_providers,
                context_exporters: &mut self.context_exporters,
                health_checks: &mut self.health_checks,
                config_providers: &mut self.config_providers,
                cli_commands: &mut self.cli_commands,
                operations: &mut self.operations,
            };

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                plugins[idx].register(&mut sink)
            }));

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    // Partial state leak fix: clear extensions from
                    // earlier successful plugins before returning error.
                    // Without this, the first plugin's extensions remain
                    // even though the method returns Err.
                    self.clear_extensions();
                    // Put pending back so caller can retry or inspect
                    self.pending = plugins;
                    return Err(PluginError::RegistrationFailed(name, e));
                }
                Err(panic_val) => {
                    // Preserve the panic message if it's a String or &str
                    let msg = panic_val
                        .downcast_ref::<String>()
                        .map(|s| s.clone())
                        .or_else(|| panic_val.downcast_ref::<&str>().map(|s| s.to_string()));

                    // Same fix: clear partial extensions from earlier
                    // successful plugins before returning the error.
                    self.clear_extensions();
                    self.pending = plugins;
                    return Err(PluginError::PanicInRegister {
                        name,
                        message: msg,
                    });
                }
            }
        }

        self.pending = plugins;
        self.initialized = true;
        Ok(())
    }

    /// Returns `true` if `resolve_and_init()` has been called successfully.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    // ── Extension query methods ──

    pub fn runtime_providers(&self) -> Vec<&dyn RuntimeProvider> {
        self.runtime_providers.iter().map(|p| p.as_ref()).collect()
    }

    pub fn context_providers(&self) -> Vec<Arc<dyn ContextProvider>> {
        self.context_providers.clone()
    }

    pub fn context_exporters(&self) -> Vec<Arc<dyn ContextExporter>> {
        self.context_exporters.clone()
    }

    pub fn health_checks(&self) -> Vec<Arc<dyn HealthCheck>> {
        self.health_checks.clone()
    }

    /// Drains all plugin runtime providers (transferring ownership).
    /// Called after init to wire providers into the Resolver.
    pub fn drain_runtime_providers(&mut self) -> Vec<Box<dyn RuntimeProvider>> {
        std::mem::take(&mut self.runtime_providers)
    }

    /// Drains all plugin configuration providers.
    pub fn drain_config_providers(&mut self) -> Vec<Box<dyn ConfigurationProvider>> {
        std::mem::take(&mut self.config_providers)
    }

    pub fn cli_commands(&self) -> Vec<&dyn CliCommand> {
        self.cli_commands.iter().map(|c| c.as_ref()).collect()
    }

    /// Drains all plugin CLI commands (transferring ownership).
    pub fn drain_cli_commands(&mut self) -> Vec<Box<dyn CliCommand>> {
        std::mem::take(&mut self.cli_commands)
    }

    pub fn operations(&self) -> Vec<&dyn Operation> {
        self.operations.iter().map(|o| o.as_ref()).collect()
    }

    /// Drains all plugin operations (transferring ownership).
    pub fn drain_operations(&mut self) -> Vec<Box<dyn Operation>> {
        std::mem::take(&mut self.operations)
    }

    /// Number of registered (pending + initialised) plugins.
    pub fn plugin_count(&self) -> usize {
        self.pending.len()
    }

    /// Clears all extension collections.
    ///
    /// Used on `resolve_and_init()` failure to prevent partial state
    /// leak: if plugin A succeeds and plugin B fails, A's extensions
    /// must not remain in the vectors after the error return.
    fn clear_extensions(&mut self) {
        self.runtime_providers.clear();
        self.context_providers.clear();
        self.context_exporters.clear();
        self.health_checks.clear();
        self.config_providers.clear();
        self.cli_commands.clear();
        self.operations.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── ExtensionSink implementation ──

impl ExtensionSink for PluginRegistry {
    fn add_runtime_provider(&mut self, provider: Box<dyn RuntimeProvider>) {
        self.runtime_providers.push(provider);
    }

    fn add_context_provider(&mut self, provider: Arc<dyn ContextProvider>) {
        self.context_providers.push(provider);
    }

    fn add_context_exporter(&mut self, exporter: Arc<dyn ContextExporter>) {
        self.context_exporters.push(exporter);
    }

    fn add_health_check(&mut self, check: Arc<dyn HealthCheck>) {
        self.health_checks.push(check);
    }

    fn add_configuration_provider(&mut self, provider: Box<dyn ConfigurationProvider>) {
        self.config_providers.push(provider);
    }

    fn add_cli_command(&mut self, cmd: Box<dyn CliCommand>) {
        self.cli_commands.push(cmd);
    }

    fn add_operation(&mut self, op: Box<dyn Operation>) {
        self.operations.push(op);
    }
}

// ── Internal registration sink (used during init to avoid aliasing) ──

struct RegistrationSink<'a> {
    runtime_providers: &'a mut Vec<Box<dyn RuntimeProvider>>,
    context_providers: &'a mut Vec<Arc<dyn ContextProvider>>,
    context_exporters: &'a mut Vec<Arc<dyn ContextExporter>>,
    health_checks: &'a mut Vec<Arc<dyn HealthCheck>>,
    config_providers: &'a mut Vec<Box<dyn ConfigurationProvider>>,
    cli_commands: &'a mut Vec<Box<dyn CliCommand>>,
    operations: &'a mut Vec<Box<dyn Operation>>,
}

impl ExtensionSink for RegistrationSink<'_> {
    fn add_runtime_provider(&mut self, provider: Box<dyn RuntimeProvider>) {
        self.runtime_providers.push(provider);
    }

    fn add_context_provider(&mut self, provider: Arc<dyn ContextProvider>) {
        self.context_providers.push(provider);
    }

    fn add_context_exporter(&mut self, exporter: Arc<dyn ContextExporter>) {
        self.context_exporters.push(exporter);
    }

    fn add_health_check(&mut self, check: Arc<dyn HealthCheck>) {
        self.health_checks.push(check);
    }

    fn add_configuration_provider(&mut self, provider: Box<dyn ConfigurationProvider>) {
        self.config_providers.push(provider);
    }

    fn add_cli_command(&mut self, cmd: Box<dyn CliCommand>) {
        self.cli_commands.push(cmd);
    }

    fn add_operation(&mut self, op: Box<dyn Operation>) {
        self.operations.push(op);
    }
}

// ── DAG topological sort with cycle detection ──

/// Performs a DFS topological sort given a list of names and an adjacency list.
///
/// Returns indices in dependency order (dependencies first).
/// Detects cycles involving 3+ interdependent plugins.
pub(crate) fn topological_sort(
    names: &[String],
    adj: &[Vec<usize>],
) -> Result<Vec<usize>, PluginError> {
    let n = names.len();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Unvisited,
        Visiting,
        Visited,
    }

    let mut state = vec![State::Unvisited; n];
    let mut order = Vec::with_capacity(n);
    let mut path: Vec<usize> = Vec::new();

    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        names: &[String],
        state: &mut Vec<State>,
        order: &mut Vec<usize>,
        path: &mut Vec<usize>,
    ) -> Result<(), PluginError> {
        match state[u] {
            State::Visiting => {
                // Found a cycle. Build the path from the first occurrence to here.
                let cycle_start = path.iter().position(|&x| x == u).unwrap_or(0);
                let cycle: Vec<String> = path[cycle_start..]
                    .iter()
                    .chain(std::iter::once(&u))
                    .map(|&i| names[i].clone())
                    .collect();
                return Err(PluginError::CyclicDependency(cycle));
            }
            State::Visited => return Ok(()),
            State::Unvisited => {}
        }

        state[u] = State::Visiting;
        path.push(u);

        for &v in &adj[u] {
            dfs(v, adj, names, state, order, path)?;
        }

        path.pop();
        state[u] = State::Visited;
        order.push(u);
        Ok(())
    }

    for i in 0..n {
        if state[i] == State::Unvisited {
            dfs(i, &adj, &names, &mut state, &mut order, &mut path)?;
        }
    }

    Ok(order)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::RuntimeProvider;

    // ── Helper: mock plugins ──

    struct TestPlugin {
        name: String,
        version: String,
        api_version: String,
        register_result: Result<(), String>,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                api_version: FORGE_PLUGIN_API_VERSION.to_string(),
                register_result: Ok(()),
            }
        }

        fn with_api(mut self, api: &str) -> Self {
            self.api_version = api.to_string();
            self
        }

        fn with_register_result(mut self, result: Result<(), String>) -> Self {
            self.register_result = result;
            self
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> &str {
            &self.version
        }
        fn api_version(&self) -> &str {
            &self.api_version
        }
        fn depends_on(&self) -> &[&str] {
            // Return references to strings stored in self.depends
            // We need to return &[&str] but we store Vec<String>
            // This is a bit awkward for testing — let's just return &[]
            // and handle dependencies via a custom approach.
            &[]
        }
        fn register(&self, _sink: &mut dyn ExtensionSink) -> Result<(), String> {
            self.register_result.clone()
        }
    }

    // ── Tests ──

    /// 4.1: Register a plugin, verify name/version, reject duplicate.
    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("test-plugin"));

        assert_eq!(plugin.name(), "test-plugin");
        assert_eq!(plugin.version(), "1.0.0");
        assert_eq!(plugin.api_version(), FORGE_PLUGIN_API_VERSION);

        // First registration should succeed
        assert!(registry.register(plugin).is_ok());

        // Second registration with same name should fail
        let duplicate = Box::new(TestPlugin::new("test-plugin"));
        let err = registry.register(duplicate).unwrap_err();
        assert!(matches!(err, PluginError::DuplicateName(_)));
        assert!(err.to_string().contains("test-plugin"));
    }

    /// 4.7: Register after init is rejected.
    #[test]
    fn test_register_after_init_rejected() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("first"));
        registry.register(plugin).unwrap();
        registry.resolve_and_init().unwrap();
        assert!(registry.is_initialized());

        // Now try to register another plugin — should be rejected
        let late = Box::new(TestPlugin::new("late"));
        let err = registry.register(late).unwrap_err();
        assert!(matches!(err, PluginError::AlreadyInitialized(_)));
        assert!(err.to_string().contains("late"));
    }

    /// 4.3: API version mismatch rejection.
    #[test]
    fn test_api_version_mismatch() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("old-plugin").with_api("0.5.0"));

        let err = registry.register(plugin).unwrap_err();
        match err {
            PluginError::ApiVersionMismatch { plugin, expected, actual } => {
                assert_eq!(plugin, "old-plugin");
                assert_eq!(expected, FORGE_PLUGIN_API_VERSION);
                assert_eq!(actual, "0.5.0");
            }
            _ => panic!("Expected ApiVersionMismatch, got: {:?}", err),
        }
    }

    /// 4.2: DAG cycle detection — 3 interdependent plugins.
    #[test]
    fn test_cycle_detection_three_plugins() {
        // A depends on B, B depends on C, C depends on A
        let names = &["A", "B", "C"].map(String::from);
        let adj = &[vec![1], vec![2], vec![0]]; // A→B, B→C, C→A
        let result = topological_sort(names, adj);

        let err = result.unwrap_err();
        match err {
            PluginError::CyclicDependency(cycle) => {
                assert!(cycle.contains(&"A".to_string()));
                assert!(cycle.contains(&"B".to_string()));
                assert!(cycle.contains(&"C".to_string()));
                // The error should mention all three
                assert!(cycle.len() >= 3);
            }
            _ => panic!("Expected CyclicDependency, got: {:?}", err),
        }
    }

    /// 4.2: Simple DAG with no cycles resolves correctly.
    #[test]
    fn test_dag_no_cycle() {
        // A depends on B, B depends on C (C has no deps)
        let names = &["A", "B", "C"].map(String::from);
        let adj = &[vec![1], vec![2], vec![]];
        let result = topological_sort(names, adj);

        assert!(result.is_ok());
        let order = result.unwrap();
        // C must come before B, B before A
        let c_pos = order.iter().position(|&i| i == 2).unwrap();
        let b_pos = order.iter().position(|&i| i == 1).unwrap();
        let a_pos = order.iter().position(|&i| i == 0).unwrap();
        assert!(c_pos < b_pos, "C must be before B");
        assert!(b_pos < a_pos, "B must be before A");
    }

    /// 4.6: Plugin panics in register() — host continues via catch_unwind.
    #[test]
    fn test_panic_in_register_is_caught() {
        struct PanicPlugin;

        impl Plugin for PanicPlugin {
            fn name(&self) -> &str {
                "panic-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, _sink: &mut dyn ExtensionSink) -> Result<(), String> {
                panic!("intentional panic in register");
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(PanicPlugin)).unwrap();

        let result = registry.resolve_and_init();
        match result {
            Err(PluginError::PanicInRegister { name, message }) => {
                assert_eq!(name, "panic-plugin");
                assert_eq!(message.as_deref(), Some("intentional panic in register"));
            }
            other => panic!("Expected PanicInRegister, got: {:?}", other),
        }
    }

    /// 4.6: Plugin returns error from register().
    #[test]
    fn test_register_error_propagated() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(
            TestPlugin::new("failing-plugin")
                .with_register_result(Err("something went wrong".to_string())),
        );
        registry.register(plugin).unwrap();

        let result = registry.resolve_and_init();
        match result {
            Err(PluginError::RegistrationFailed(name, reason)) => {
                assert_eq!(name, "failing-plugin");
                assert!(reason.contains("something went wrong"));
            }
            other => panic!("Expected RegistrationFailed, got: {:?}", other),
        }
    }

    /// scan_directory returns Ok (stub).
    #[test]
    fn test_scan_directory_stub() {
        let result = PluginRegistry::scan_directory(std::path::Path::new("/tmp/nonexistent"));
        assert!(result.is_ok());
    }

    /// 4.4: Integration — register plugin, init, query extension types.
    #[test]
    fn test_register_and_query_extensions() {
        // A plugin that registers a mock runtime provider via ExtensionSink
        struct ExtPlugin;

        impl Plugin for ExtPlugin {
            fn name(&self) -> &str {
                "ext-plugin"
            }
            fn version(&self) -> &str {
                "2.0.0"
            }
            fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String> {
                // Register a mock runtime provider
                sink.add_runtime_provider(Box::new(MockRuntimeProvider {
                    name: "custom-runtime".to_string(),
                }));
                // Register a mock CLI command
                sink.add_cli_command(Box::new(MockCliCommand {
                    cmd_name: "mycmd".to_string(),
                }));
                Ok(())
            }
        }

        // Mock types used by ExtPlugin
        struct MockRuntimeProvider {
            name: String,
        }

        impl RuntimeProvider for MockRuntimeProvider {
            fn name(&self) -> &str {
                &self.name
            }
            fn resolve(
                &self,
                _version_req: &str,
                _platform: &str,
                _arch: &str,
                _registry: &crate::registry::HybridRegistry,
            ) -> Result<crate::types::RuntimeLock, String> {
                Err("mock not implemented".to_string())
            }
        }

        struct MockCliCommand {
            cmd_name: String,
        }

        impl CliCommand for MockCliCommand {
            fn name(&self) -> &str {
                &self.cmd_name
            }
            fn description(&self) -> &str {
                "A mock plugin command"
            }
            fn execute(&self, _args: &[String]) -> Result<(), String> {
                Ok(())
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(ExtPlugin)).unwrap();
        registry.resolve_and_init().unwrap();

        // Query extension types
        let providers = registry.runtime_providers();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name(), "custom-runtime");

        let cmds = registry.cli_commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "mycmd");

        // Also drain and verify ownership transfer
        let drained_cmds = registry.drain_cli_commands();
        assert_eq!(drained_cmds.len(), 1);
        assert_eq!(drained_cmds[0].name(), "mycmd");
    }

    /// 4.5: CliCommand dispatch via registry.
    #[test]
    fn test_cli_command_dispatch() {
        struct HelloCmd;

        impl Plugin for HelloCmd {
            fn name(&self) -> &str {
                "hello-plugin"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            fn register(&self, sink: &mut dyn ExtensionSink) -> Result<(), String> {
                sink.add_cli_command(Box::new(HelloCliCommand));
                Ok(())
            }
        }

        struct HelloCliCommand;

        impl CliCommand for HelloCliCommand {
            fn name(&self) -> &str {
                "hello"
            }
            fn description(&self) -> &str {
                "Say hello"
            }
            fn execute(&self, args: &[String]) -> Result<(), String> {
                if args.is_empty() {
                    Ok(()) // just hello
                } else {
                    Err(format!("unexpected args: {:?}", args))
                }
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(HelloCmd)).unwrap();
        registry.resolve_and_init().unwrap();

        let cmds = registry.cli_commands();
        let cmd = cmds.iter().find(|c| c.name() == "hello").unwrap();
        assert_eq!(cmd.name(), "hello");
        assert_eq!(cmd.description(), "Say hello");
        assert!(cmd.execute(&[]).is_ok());
        assert!(cmd.execute(&["--flag".to_string(), "value".to_string()]).is_err());
    }

    /// Test that the topological sort function handles no-dependency case.
    #[test]
    fn test_no_dependencies() {
        let names = &["A", "B", "C"].map(String::from);
        let adj = &[vec![], vec![], vec![]];
        let result = topological_sort(names, adj);
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.len(), 3);
    }

    /// Test init without any registered plugins is a no-op.
    #[test]
    fn test_init_empty_registry() {
        let mut registry = PluginRegistry::new();
        assert!(registry.resolve_and_init().is_ok());
        assert!(registry.is_initialized());
    }
}
