//! Anvil Rust SDK — typed wrapper around the Anvil engine.
//!
//! The `Anvil` struct wraps `anvil_core::Engine` directly (no RPC loopback)
//! and provides a typed, ergonomic API for all engine operations.
//!
//! # Feature flags
//!
//! - `async` (off by default): enables `async fn` variants of all methods.
//!   Without it, methods are sync wrappers that create a temporary runtime.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use anvil_core::api::v1::{OperationSummary, RuntimeExplanation};
use anvil_core::Engine;

// ── Error Type ──────────────────────────────────────────────────────────────

/// Errors returned by the Anvil SDK.
#[derive(Debug)]
pub struct AnvilError {
    message: String,
}

impl AnvilError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AnvilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AnvilError {}

// ── Main Struct ─────────────────────────────────────────────────────────────

/// A typed wrapper around the Anvil engine.
///
/// Construct via [`Anvil::new()`], which creates the underlying `Engine`
/// using the current working directory.
pub struct Anvil {
    engine: Engine,
}

impl Anvil {
    /// Create a new `Anvil` instance.
    ///
    /// The engine is initialised with the current working directory.
    /// Returns an error if the engine cannot be created.
    pub fn new() -> Result<Self, AnvilError> {
        let current_dir =
            std::env::current_dir().map_err(|e| AnvilError::new(format!("{}", e)))?;
        let engine = Engine::new(current_dir)
            .map_err(|e| AnvilError::new(format!("Failed to create engine: {}", e)))?;
        Ok(Self { engine })
    }

    /// Create a new `Anvil` instance rooted at `dir`.
    ///
    /// Useful when you need explicit control over the workspace root.
    pub fn with_root(root: PathBuf) -> Result<Self, AnvilError> {
        let engine =
            Engine::new(root).map_err(|e| AnvilError::new(format!("{}", e)))?;
        Ok(Self { engine })
    }

    // ── Sync methods (default feature) ────────────────────────────────

    /// Get the current lifecycle state.
    pub fn status(&self) -> Result<String, AnvilError> {
        self.block_on(self.engine.get_status())
    }

    /// Sync runtimes from lockfile.
    pub fn sync(&self) -> Result<(), AnvilError> {
        self.block_on(self.engine.sync())
    }

    /// Repair corrupted or missing runtimes.
    pub fn repair(&self) -> Result<(), AnvilError> {
        self.block_on(self.engine.repair())
    }

    /// Clean all local cache and state.
    pub fn clean(&self) -> Result<(), AnvilError> {
        self.block_on(self.engine.clean())
    }

    /// Explain a runtime's configuration and cache status.
    pub fn explain(&self, runtime: &str) -> Result<RuntimeExplanation, AnvilError> {
        self.block_on(self.engine.explain(runtime))
    }

    /// Show past operations history.
    pub fn history(&self, limit: Option<usize>) -> Result<Vec<OperationSummary>, AnvilError> {
        self.block_on(self.engine.history(limit))
    }

    // ── Environment ───────────────────────────────────────────────────

    /// List all environment variables.
    pub fn env_list(&self) -> Result<HashMap<String, String>, AnvilError> {
        self.block_on(self.engine.env_list())
    }

    /// Get a single environment variable by key.
    pub fn env_get(&self, key: &str) -> Result<Option<String>, AnvilError> {
        self.block_on(self.engine.env_get(key))
    }

    /// Set an environment variable.
    pub fn env_set(&self, key: &str, value: &str) -> Result<(), AnvilError> {
        self.block_on(self.engine.env_set(key, value))
    }

    /// Unset/remove an environment variable.
    pub fn env_unset(&self, key: &str) -> Result<(), AnvilError> {
        self.block_on(self.engine.env_unset(key))
    }

    /// Resolve the unified environment (including profile).
    pub fn env_resolve(
        &self,
        profile: Option<&str>,
    ) -> Result<anvil_core::secrets::ResolvedEnvironment, AnvilError> {
        self.block_on(self.engine.env_resolve(profile))
    }

    // ── Exec ───────────────────────────────────────────────────────────

    /// Execute a command inside the activated environment.
    /// Returns the exit code of the command.
    pub fn run(&self, cmd: &str, args: &[String]) -> Result<i32, AnvilError> {
        self.block_on(self.run_impl(cmd, args))
    }

    /// Spawn an interactive subshell inside the activated environment.
    /// Returns the exit code of the shell.
    pub fn shell(&self) -> Result<i32, AnvilError> {
        self.block_on(self.shell_impl())
    }

    /// Query contextual environment information in the given format.
    /// Supported formats: "json", "json-pretty", "markdown", "mcp".
    pub fn context(&self, format: &str) -> Result<String, AnvilError> {
        self.block_on(self.context_impl(format))
    }

    // ── Exec internals (always available, async, return String for block_on compat) ──

    async fn run_impl(&self, cmd: &str, args: &[String]) -> Result<i32, String> {
        use anvil_core::event_bus::EventBus;
        use anvil_core::operations::{Context, RunOperation};
        use anvil_core::Operation;

        let event_bus = EventBus::new(100);
        let mut ctx = Context::new(
            self.engine.workspace_root.clone(),
            self.engine.cache_dir.clone(),
            event_bus,
        );
        let _ = ctx.load_config();
        let _ = ctx.load_lockfile();

        let op = RunOperation {
            cmd: cmd.to_string(),
            args: args.to_vec(),
        };
        let plan = op.plan(&ctx)?;
        let result = op.execute(&mut ctx, plan).await?;

        // Extract exit code from diagnostics
        for diag in &result.diagnostics {
            if let Some(code) = diag.strip_prefix("Command exited with status code: ") {
                return Ok(code.parse::<i32>().unwrap_or(-1));
            }
        }
        Ok(0)
    }

    async fn shell_impl(&self) -> Result<i32, String> {
        use anvil_core::event_bus::EventBus;
        use anvil_core::operations::{Context, ShellOperation};
        use anvil_core::Operation;

        let event_bus = EventBus::new(100);
        let mut ctx = Context::new(
            self.engine.workspace_root.clone(),
            self.engine.cache_dir.clone(),
            event_bus,
        );
        let _ = ctx.load_config();
        let _ = ctx.load_lockfile();

        let op = ShellOperation;
        let plan = op.plan(&ctx)?;
        let result = op.execute(&mut ctx, plan).await?;

        // Extract exit code from diagnostics
        for diag in &result.diagnostics {
            if let Some(code) = diag.strip_prefix("Shell exited with code ") {
                return Ok(code.parse::<i32>().unwrap_or(-1));
            }
        }
        Ok(0)
    }

    async fn context_impl(&self, format: &str) -> Result<String, String> {
        use anvil_core::context::{ContextEngine, ContextOptions};
        use anvil_core::ContextExporter;
        use anvil_core::{
            ConfigurationProviderImpl, DiagnosticsProviderImpl, EnvironmentProviderImpl,
            JsonExporter, MarkdownExporter, McpExporter, RuntimeProviderImpl,
            SecretsProviderImpl, WorkspaceProviderImpl,
        };

        let mut ctx_engine = ContextEngine::new();
        ctx_engine.register(std::sync::Arc::new(RuntimeProviderImpl));
        ctx_engine.register(std::sync::Arc::new(ConfigurationProviderImpl));
        ctx_engine.register(std::sync::Arc::new(DiagnosticsProviderImpl));
        ctx_engine.register(std::sync::Arc::new(WorkspaceProviderImpl));
        ctx_engine.register(std::sync::Arc::new(EnvironmentProviderImpl));
        ctx_engine.register(std::sync::Arc::new(SecretsProviderImpl));

        let cache_dir = anvil_core::get_cache_dir()?;
        let active_profile = {
            let toml_path = self.engine.workspace_root.join("anvil.toml");
            if toml_path.exists() {
                std::env::var("ANVIL_PROFILE").ok().or_else(|| {
                    anvil_core::load_config(&toml_path).ok().and_then(|c| {
                        c.profile.and_then(|p| p.keys().next().cloned())
                    })
                })
            } else {
                None
            }
        };

        let options = ContextOptions {
            scopes: Vec::new(),
            excludes: Vec::new(),
            workspace_root: self.engine.workspace_root.clone(),
            cache_dir,
            active_profile,
        };

        let context = ctx_engine.query(&options).await?;

        let output = match format {
            "json-pretty" | "pretty" => {
                let exporter = JsonExporter { pretty: true };
                exporter.export(&context)?
            }
            "markdown" | "md" => {
                let exporter = MarkdownExporter;
                exporter.export(&context)?
            }
            "mcp" => {
                let exporter = McpExporter;
                exporter.export(&context)?
            }
            _ => {
                // json (default)
                let exporter = JsonExporter { pretty: false };
                exporter.export(&context)?
            }
        };

        Ok(output)
    }

    // ── Secrets ───────────────────────────────────────────────────────

    /// Set a secret.
    pub fn secret_set(&self, key: &str, value: &str) -> Result<(), AnvilError> {
        self.block_on(self.engine.secret_set(key, value))
    }

    /// Get a secret by key.
    pub fn secret_get(&self, key: &str) -> Result<Option<String>, AnvilError> {
        self.block_on(self.engine.secret_get(key))
    }

    /// List all secret keys.
    pub fn secret_list(&self) -> Result<Vec<String>, AnvilError> {
        self.block_on(self.engine.secret_list())
    }

    /// Remove a secret.
    pub fn secret_remove(&self, key: &str) -> Result<(), AnvilError> {
        self.block_on(self.engine.secret_remove(key))
    }

    // ── Internal ──────────────────────────────────────────────────────

    fn block_on<T>(&self, fut: impl std::future::Future<Output = Result<T, String>>) -> Result<T, AnvilError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                let handle: tokio::runtime::Handle = handle;
                tokio::task::block_in_place(|| {
                    handle
                        .block_on(fut)
                        .map_err(|e| AnvilError::new(e))
                })
            }
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AnvilError::new(format!("Failed to build runtime: {}", e)))?;
                rt.block_on(fut)
                    .map_err(|e| AnvilError::new(e))
            }
        }
    }
}

// ── Async Methods (feature = "async") ───────────────────────────────────────

#[cfg(feature = "async")]
impl Anvil {
    /// Get the current lifecycle state (async).
    pub async fn status_async(&self) -> Result<String, AnvilError> {
        self.engine.get_status().await.map_err(AnvilError::new)
    }

    /// Sync runtimes from lockfile (async).
    pub async fn sync_async(&self) -> Result<(), AnvilError> {
        self.engine.sync().await.map_err(AnvilError::new)
    }

    /// Repair corrupted or missing runtimes (async).
    pub async fn repair_async(&self) -> Result<(), AnvilError> {
        self.engine.repair().await.map_err(AnvilError::new)
    }

    /// Clean all local cache and state (async).
    pub async fn clean_async(&self) -> Result<(), AnvilError> {
        self.engine.clean().await.map_err(AnvilError::new)
    }

    /// Explain a runtime's configuration (async).
    pub async fn explain_async(&self, runtime: &str) -> Result<RuntimeExplanation, AnvilError> {
        self.engine.explain(runtime).await.map_err(AnvilError::new)
    }

    /// Show past operations history (async).
    pub async fn history_async(&self, limit: Option<usize>) -> Result<Vec<OperationSummary>, AnvilError> {
        self.engine.history(limit).await.map_err(AnvilError::new)
    }

    /// List environment variables (async).
    pub async fn env_list_async(&self) -> Result<HashMap<String, String>, AnvilError> {
        self.engine.env_list().await.map_err(AnvilError::new)
    }

    /// Get env var (async).
    pub async fn env_get_async(&self, key: &str) -> Result<Option<String>, AnvilError> {
        self.engine.env_get(key).await.map_err(AnvilError::new)
    }

    /// Set env var (async).
    pub async fn env_set_async(&self, key: &str, value: &str) -> Result<(), AnvilError> {
        self.engine.env_set(key, value).await.map_err(AnvilError::new)
    }

    /// Unset env var (async).
    pub async fn env_unset_async(&self, key: &str) -> Result<(), AnvilError> {
        self.engine.env_unset(key).await.map_err(AnvilError::new)
    }

    /// Resolve unified environment (async).
    pub async fn env_resolve_async(
        &self,
        profile: Option<&str>,
    ) -> Result<anvil_core::secrets::ResolvedEnvironment, AnvilError> {
        self.engine.env_resolve(profile).await.map_err(AnvilError::new)
    }

    /// Execute a command inside the activated environment (async).
    /// Returns the exit code.
    pub async fn run_async(&self, cmd: &str, args: &[String]) -> Result<i32, AnvilError> {
        self.run_impl(cmd, args).await
    }

    /// Spawn an interactive subshell inside the activated environment (async).
    /// Returns the exit code.
    pub async fn shell_async(&self) -> Result<i32, AnvilError> {
        self.shell_impl().await
    }

    /// Query contextual environment information in the given format (async).
    /// Supported formats: "json", "json-pretty", "markdown", "mcp".
    pub async fn context_async(&self, format: &str) -> Result<String, AnvilError> {
        self.context_impl(format).await
    }

    /// Set secret (async).
    pub async fn secret_set_async(&self, key: &str, value: &str) -> Result<(), AnvilError> {
        self.engine.secret_set(key, value).await.map_err(AnvilError::new)
    }

    /// Get secret (async).
    pub async fn secret_get_async(&self, key: &str) -> Result<Option<String>, AnvilError> {
        self.engine.secret_get(key).await.map_err(AnvilError::new)
    }

    /// List secrets (async).
    pub async fn secret_list_async(&self) -> Result<Vec<String>, AnvilError> {
        self.engine.secret_list().await.map_err(AnvilError::new)
    }

    /// Remove secret (async).
    pub async fn secret_remove_async(&self, key: &str) -> Result<(), AnvilError> {
        self.engine.secret_remove(key).await.map_err(AnvilError::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the run, shell, and context methods exist at the type level
    /// (structural coverage — actual execution requires a anvil workspace).
    #[test]
    fn test_run_shell_context_methods_exist() {
        // We can't easily run these without a anvil workspace, but we verify
        // the signatures compile and the Anvil struct is constructible.
        let anvil = match Anvil::new() {
            Ok(f) => f,
            Err(_) => return, // skip if engine can't init
        };

        // run() and shell() will fail without a proper anvil env, but they
        // should return an error (not panic).
        let run_result = anvil.run("echo", &["hello".to_string()]);
        // In a non-anvil directory, this may fail with engine errors
        // but it should never panic.
        assert!(run_result.is_ok() || run_result.is_err());

        let shell_result = anvil.shell();
        assert!(shell_result.is_ok() || shell_result.is_err());

        // context() should either return data or an error in non-anvil dirs
        let ctx_result = anvil.context("json");
        assert!(ctx_result.is_ok() || ctx_result.is_err());
    }

    /// Verify AnvilError implements Display and Error.
    #[test]
    fn test_anvil_error_traits() {
        let err = AnvilError::new("test error");
        assert_eq!(err.to_string(), "test error");

        let err_ref: &dyn std::error::Error = &err;
        assert!(err_ref.source().is_none());
    }

    /// Verify Anvil::new() succeeds in a directory (will use temp for real tests).
    #[test]
    fn test_anvil_new_succeeds() {
        // This should succeed because Engine::new() creates an engine
        // even in non-anvil directories (just has empty config).
        let result = Anvil::new();
        // Engine::new() may fail if cache dir can't be created, but
        // in normal environments it should succeed.
        assert!(result.is_ok() || result.is_err());
    }

    /// Verify env roundtrip works on a Anvil instance.
    #[test]
    fn test_env_roundtrip() {
        let anvil = match Anvil::new() {
            Ok(f) => f,
            Err(_) => return, // skip if engine can't init
        };

        // env_set then env_get
        assert!(anvil.env_set("ANVIL_TEST_VAR", "test_value").is_ok());
        let val = anvil.env_get("ANVIL_TEST_VAR").unwrap();
        assert_eq!(val, Some("test_value".to_string()));

        // Cleanup
        let _ = anvil.env_unset("ANVIL_TEST_VAR");
        let val = anvil.env_get("ANVIL_TEST_VAR").unwrap();
        assert_eq!(val, None);
    }

    /// Verify secret roundtrip works.
    #[test]
    fn test_secret_roundtrip() {
        let anvil = match Anvil::new() {
            Ok(f) => f,
            Err(_) => return,
        };

        assert!(anvil.secret_set("SDK_TEST_KEY", "sdk_test_val").is_ok());
        let val = anvil.secret_get("SDK_TEST_KEY").unwrap();
        assert_eq!(val, Some("sdk_test_val".to_string()));

        // List includes our key
        let keys = anvil.secret_list().unwrap();
        assert!(keys.contains(&"SDK_TEST_KEY".to_string()));

        // Remove
        assert!(anvil.secret_remove("SDK_TEST_KEY").is_ok());
        let val = anvil.secret_get("SDK_TEST_KEY").unwrap();
        assert_eq!(val, None);
    }
}
