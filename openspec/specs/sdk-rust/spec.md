# SDK Rust Specification

## Purpose

Defines the official Rust SDK crate (`forge-sdk`) that wraps Engine operations in a typed, ergonomic API. This is the reference SDK — all other SDKs mirror its method surface via JSON-RPC.

## Requirements

### Requirement: Crate Structure

The `forge-sdk` crate MUST be a workspace member at `crates/forge-sdk/`. It MUST depend on `forge-core` and `serde_json`. It MUST NOT expose `forge-core` internals in its public API.

#### Scenario: Crate compiles as workspace member

- GIVEN the forge workspace
- WHEN `cargo build -p forge-sdk` is run
- THEN it MUST compile without errors
- AND `forge-core` MUST NOT appear in the public re-exports of `forge-sdk`

### Requirement: Forge Struct

The SDK MUST provide a `Forge` struct constructed via `Forge::new() -> Result<Self, ForgeError>`. The constructor MUST NOT require configuration arguments (defaults are sufficient).

#### Scenario: Create Forge instance

- GIVEN no prior Forge state
- WHEN `Forge::new()` is called
- THEN it MUST return `Ok(Forge)` without errors

### Requirement: Method Surface

The `Forge` struct MUST implement these methods, all returning `Result<T, ForgeError>`:

- `status() -> StatusInfo`
- `sync() -> SyncReport`
- `repair() -> RepairReport`
- `clean() -> CleanReport`
- `run(cmd: &str, args: &[&str]) -> RunOutput`
- `shell() -> InteractiveSession`
- `context(format: ContextFormat) -> ContextData`
- `explain(runtime: &str) -> String`
- `history(limit: usize) -> Vec<HistoryEntry>`
- `env_list() -> Vec<EnvVar>`, `env_get(key) -> Option<String>`, `env_set(key, val)`, `env_unset(key)`, `env_resolve(key) -> String`
- `secret_set(key, val)`, `secret_get(key) -> Option<String>`, `secret_list() -> Vec<String>`, `secret_remove(key)`

#### Scenario: Sync environment

- GIVEN a `Forge` instance
- WHEN `sync()` is called
- THEN it MUST return a `SyncReport` with success/failure details

#### Scenario: Query context

- GIVEN a `Forge` instance
- WHEN `context(ContextFormat::Json)` is called
- THEN it MUST return structured `ContextData`

#### Scenario: Manage secrets

- GIVEN a `Forge` instance
- WHEN `secret_set("TOKEN", "abc123")` and `secret_get("TOKEN")` are called
- THEN `secret_get` MUST return `Some("abc123")`
- AND `secret_remove("TOKEN")` MUST make `secret_get("TOKEN")` return `None`

### Requirement: Async Support

The SDK SHOULD provide both sync and async (`async fn`) versions of all methods via a feature flag `async`.

#### Scenario: Async compile with feature

- GIVEN a project depending on `forge-sdk` with feature `async` enabled
- WHEN `cargo check` is run
- THEN async methods MUST be accessible and tokio-compatible

### Requirement: Error Handling

`ForgeError` MUST implement `std::fmt::Display` and `std::error::Error`. It SHOULD wrap a human-readable message string.

#### Scenario: Error propagation

- GIVEN an invalid method call on `Forge`
- WHEN the call returns `Err(e)`
- THEN `e.to_string()` MUST produce a non-empty string
- AND `e` MUST satisfy `std::error::Error` trait bounds
