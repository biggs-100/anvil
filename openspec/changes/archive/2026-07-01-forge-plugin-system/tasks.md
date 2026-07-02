# Tasks: Forge Plugin System

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 700–1300 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Core + Registry) → PR 2 (Integrations + CLI) |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Plugin trait, PluginRegistry, DAG, API gating, Engine hook | PR 1 | base=main; unit-tested, standalone |
| 2 | 7 integration points + CLI command merge | PR 2 | depends on PR 1; tests included |

## Phase 1: Foundation

- [x] 1.1 Create `crates/forge-core/src/plugin/mod.rs` — `Plugin` trait, `CliCommand` trait, `FORGE_PLUGIN_API_VERSION`, `PluginError` enum, pub re-exports
- [x] 1.2 Create `crates/forge-core/src/plugin/registry.rs` — `PluginRegistry` struct with `register()`, `scan_directory()` stub (returns Ok), query-by-type accessors
- [x] 1.3 Add `pub mod plugin` and re-exports to `crates/forge-core/src/lib.rs`

## Phase 2: Core Implementation

- [x] 2.1 Add API version gate in `PluginRegistry::register()` — reject on `api_version` mismatch with plugin name + expected + actual in error
- [x] 2.2 Implement `resolve_and_init()` — DFS topological sort with cycle detection (3+ interdependent plugins), `catch_unwind` per plugin `register()`
- [x] 2.3 Add `Engine::register_plugin()` and plugin-aware constructor to `crates/forge-core/src/api/v1.rs`

## Phase 3: Integration / Wiring

- [x] 3.1 Modify `crates/forge-core/src/resolver.rs` — query plugin `RuntimeProvider`s alongside built-in; built-in wins on name conflict
- [x] 3.2 Modify `crates/forge-core/src/context/mod.rs` — `ContextEngine` queries plugin `ContextProvider`s and `ContextExporter`s; skip failed providers, reject duplicate exporter names
- [x] 3.3 Modify `crates/forge-core/src/diagnostics/mod.rs` — `DiagnosticEngine::with_checks()` accepts plugin `HealthCheck`s; Fast/Deep mode filtering applies
- [x] 3.4 Modify `crates/forge-core/src/secrets/mod.rs` — insert plugin `ConfigurationProvider` as level 2.5 in config stack (between forge.local.toml and forge.secrets)
- [x] 3.5 Modify `crates/forge-core/src/operations/mod.rs` — refactored `Operation` trait: `async fn execute` → `fn execute<'a>(...) -> OperationFuture<'a>` (BoxFuture), making it `dyn`-compatible; `ExtensionSink` now has `add_operation()`; `PluginRegistry` has `operations()` and `drain_operations()`
- [x] 3.6 Modify `crates/forge-cli/src/main.rs` — load `CliCommand` from registry at startup, merge into command table, enforce built-in precedence

## Phase 4: Testing

- [x] 4.1 Unit: register plugin, verify name/version/api_version, reject duplicate
- [x] 4.2 Unit: DAG cycle detection — 3 interdependent plugins trigger cycle error naming all three
- [x] 4.3 Unit: API version mismatch rejection with expected/actual in error
- [x] 4.4 Integration: `Engine::register_plugin()` → resolve → init → query extension types
- [x] 4.5 CLI integration: register `CliCommand`, verify `forge mycmd` dispatch; duplicate name rejected with warning
- [x] 4.6 Plugin panics in `register()` — verify host continues via `catch_unwind`
