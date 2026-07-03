# Design: Anvil Plugin System

## Technical Approach

New `plugin/` module in anvil-core with a `Plugin` trait, `PluginRegistry`, and `CliCommand` trait. Plugins are Rust trait objects registered at compile time — either as workspace members or programmatically via `Engine::register_plugin()`. The registry performs API version gating, DAG dependency resolution, and topological init. Each plugin's `register()` hook injects its extension types (RuntimeProvider, ContextProvider, HealthCheck, etc.) into the registry. The Engine collects these at startup alongside built-in implementations, which always take precedence. All 7 extension types share a single loading path through PluginRegistry.

> **Future path**: `scan_directory()` is designed to later support dynamic loading via `libloading` when third-party plugins are needed. The trait, registry, and lifecycle remain unchanged — only the loading mechanism swaps.

## Architecture Decisions

| Option | Tradeoffs | Decision |
|--------|-----------|----------|
| Dynamic `.so`/`.dll` loading vs. static-only | Dynamic enables third-party plugins without recompiling. Requires `libloading` and FFI boundary complexity. Static keeps it simple, safe, and platform-independent. | **Static (compile-time)** — `Box<dyn Plugin>` via workspace members or programmatic registration. Dynamic loading deferred until third-party demand materializes. |
| Single `Plugin` trait with `register()` vs. one trait per extension type | Single trait: one factory per plugin, simpler lifecycle. Per-trait: more granular but 7 separate load paths. | **Single Plugin trait** — `register(&self, &mut PluginRegistry)` adds all extensions in one call |
| PluginRegistry standalone vs. owned by Engine | Engine-ownership centralizes lifecycle; aligns with existing facade pattern. | **Engine owns PluginRegistry** — accessed via `engine.plugin_registry()` |
| Topo sort crate vs. manual DFS | Crate dep vs. ~40 lines DFS + cycle detection. | **Manual DFS** — no new dep, simple algorithm |
| `catch_unwind` per plugin vs. Result propagation | catch_unwind isolates panics; Result can't catch stack unwinding. | **catch_unwind** — host crash is unacceptable |
| CliCommand in registry vs. module lookup | Registry: commands discoverable centrally. Module lookup: CLI must iterate each plugin. | **CliCommand in PluginRegistry** — `get_cli_commands()` returns all registered commands |

## Data Flow

```
Engine::new() → Programmatic plugin registration(s)
  ├─→ register(name, Box<dyn Plugin>)  [workspace members or SDK]
  └─→ PluginRegistry::new()
       └─→ resolve_dag() → topological sort
            └─→ for each plugin: catch_unwind { plugin.register(&mut registry) }
                 ├─→ RuntimeProvider → Resolver
                 ├─→ ContextProvider/Exporter → ContextEngine
                 ├─→ HealthCheck → DiagnosticEngine (via with_checks())
                 ├─→ ConfigurationProvider → config stack level 2.5
                 ├─→ Operation → Engine dispatch fallback
                 └─→ CliCommand → CLI command merge (built-in wins)

[Future] scan_directory() → dynamic loading via libloading
  Same flow above, just different loading source

Engine::sync/repair/clean: checks plugin ops before built-in fallback
Context command: built-in providers (6) + plugin providers via registry
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/src/plugin/mod.rs` | Create | Plugin trait, CliCommand trait, ANVIL_PLUGIN_API_VERSION, re-exports |
| `crates/anvil-core/src/plugin/registry.rs` | Create | PluginRegistry: register, scan, resolve_dag, init, query-by-type |
| `crates/anvil-core/Cargo.toml` | Modify | Add no new deps — pure trait objects only |
| `crates/anvil-core/src/lib.rs` | Modify | Add `pub mod plugin`, re-exports |
| `crates/anvil-core/src/api/v1.rs` | Modify | Engine gains `register_plugin()`, plugin-aware constructors |
| `crates/anvil-core/src/resolver.rs` | Modify | Resolver accepts plugin RuntimeProviders from registry |
| `crates/anvil-core/src/context/mod.rs` | Modify | ContextEngine accepts plugin providers/exporters |
| `crates/anvil-core/src/diagnostics/mod.rs` | Modify | DiagnosticEngine accepts plugin HealthChecks via `with_checks()` |
| `crates/anvil-core/src/secrets/mod.rs` | Modify | Config resolution adds plugin ConfigurationProvider at level 2.5 |
| `crates/anvil-core/src/operations/mod.rs` | Modify | Operation dispatch checks plugin ops as fallback |
| `crates/anvil-cli/src/main.rs` | Modify | Plugin loading at startup, CliCommand merge, built-in precedence |

## Interfaces / Contracts

```rust
pub const ANVIL_PLUGIN_API_VERSION: &str = "1.0.0";

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn api_version(&self) -> &str { ANVIL_PLUGIN_API_VERSION }
    fn depends_on(&self) -> &[&str] { &[] }
    fn register(&self, registry: &mut PluginRegistry) -> Result<(), String>;
}

pub trait CliCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> Result<(), String>;
}

pub struct PluginRegistry { /* plugins, deps, extension maps */ }
impl PluginRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError>;
    pub fn scan_directory(path: &Path) -> Result<(), PluginError>;
    pub fn resolve_and_init(&mut self) -> Result<(), PluginError>;
    // Extension queries
    pub fn runtime_providers(&self) -> Vec<&dyn RuntimeProvider>;
    pub fn context_providers(&self) -> Vec<Arc<dyn ContextProvider>>;
    pub fn context_exporters(&self) -> Vec<Arc<dyn ContextExporter>>;
    pub fn health_checks(&self) -> Vec<Arc<dyn HealthCheck>>;
    pub fn config_providers(&self) -> Vec<Box<dyn ConfigurationProvider>>;
    pub fn operations(&self) -> Vec<&dyn Operation>;
    pub fn cli_commands(&self) -> Vec<&dyn CliCommand>;
}
```

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | Plugin trait, registry ops | Test register, name/version/api_version, duplicate detection |
| Unit | DAG cycle detection | 3 interdependent plugins, verify cycle error |
| Unit | API version mismatch | Wrong `api_version`, verify rejection message |
| Unit | Built-in precedence | Plugin RuntimeProvider with same name as built-in, verify built-in wins |
| Integration | Engine::register_plugin flow | Register → init → query extension types |
| Integration | CLI command merge | Register CliCommand, verify dispatch; register duplicate, verify rejection |
| E2E | Filesystem scan | Place test `.so` in temp dir, scan, verify loaded |
| E2E | Error isolation | Plugin panics in register(), verify host continues |

## Migration / Rollout

No migration required — new module only. Existing core code paths unchanged. Rollback: revert `lib.rs` module addition and CLI integration; core continues without plugin module.

## Open Questions

- [ ] None — compile-time loading avoids FFI/platform concerns entirely. Dynamic loading questions deferred.
