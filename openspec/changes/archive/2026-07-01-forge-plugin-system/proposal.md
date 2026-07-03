# Proposal: Anvil Plugin System

## Intent

Allow Anvil to grow without modifying core. All extension through trait-based plugins with a shared loading, registration, version-gating, and dependency-resolution mechanism.

## Scope

### In Scope
- Plugin registry: scanning `~/.anvil/plugins/`, programmatic `Engine::register_plugin()`, API version check, DAG dependency resolution
- Trait wrappers for all 7 extension types
- CLI plugin loading at startup
- Error handling with clear rejection messages

### Out of Scope
- WASM or dynamic linking — compile-time trait objects only
- Plugin sandboxing, resource limits, marketplace, hot-reload

## Capabilities

### New Capabilities
- `plugin-registry`: Core loading, scanning, API version validation, dependency graph, registration lifecycle
- `plugin-cli-command`: Trait and registration for third-party CLI commands injected at startup

### Modified Capabilities
- `runtime-providers`: Accept plugin-registered `RuntimeProvider` impls
- `config-engine`: Accept plugin-registered `ConfigurationProvider` impls
- `context-providers`: Accept plugin-registered `ContextProvider` impls
- `context-exporters`: Accept plugin-registered `ContextExporter` impls
- `operations-layer`: Accept plugin-registered `Operation` impls
- `diagnostic-checks`: Accept plugin-registered `HealthCheck` impls

## Approach

New `crates/anvil-core/src/plugin/` module with:
1. `Plugin` trait (name, version, api_version, depends_on, register)
2. `PluginRegistry` (scan, load, resolve deps, initialize)
3. Integration points in `Engine`, `Resolver`, `DiagnosticEngine`, `ContextEngine`, CLI
4. `ANVIL_PLUGIN_API_VERSION` constant for version gating

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/plugin/` | New | Plugin registry module |
| `crates/anvil-core/src/lib.rs` | Modified | Add `pub mod plugin`, re-exports |
| `crates/anvil-core/src/resolver.rs` | Modified | `Resolver` accepts plugin RuntimeProviders |
| `crates/anvil-core/src/context/mod.rs` | Modified | ContextEngine accepts plugin providers/exporters |
| `crates/anvil-core/src/operations/mod.rs` | Modified | Engine accepts plugin Operations |
| `crates/anvil-core/src/diagnostics/mod.rs` | Modified | DiagnosticEngine accepts plugin HealthChecks |
| `crates/anvil-core/src/secrets/mod.rs` | Modified | Accept plugin ConfigurationProviders |
| `crates/anvil-cli/src/main.rs` | Modified | Plugin loading at startup |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| DAG cycle between plugins | Low | Simple `depends_on` + DFS cycle check |
| API version mismatch | Low | Exact-match gate with clear error message |
| Plugin trait maintenance burden | Medium | All 7 share same loading path; thin traits |

## Rollback Plan

Revert module addition in `lib.rs` and CLI startup integration. Core continues without plugin module — all existing code paths unchanged.

## Dependencies

None — pure Rust trait objects, no new crate dependencies.

## Success Criteria

- [ ] `PluginRegistry` scans `~/.anvil/plugins/` and loads trait objects
- [ ] API version mismatch is rejected with a clear error
- [ ] DAG cycle detection works for 3+ interdependent plugins
- [ ] All 7 extension types can be registered and function correctly
- [ ] All existing tests pass without modification
