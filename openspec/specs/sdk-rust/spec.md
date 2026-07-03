# SDK Rust Specification

## Purpose

Defines the official Rust SDK crate (`anvil-sdk`) that wraps Engine operations in a typed, ergonomic API. This is the reference SDK — all other SDKs mirror its method surface via JSON-RPC.

## Requirements

### Requirement: Crate Structure

The `anvil-sdk` crate MUST be a workspace member at `crates/anvil-sdk/`. It MUST depend on `anvil-core` and `serde_json`. It MUST NOT expose `anvil-core` internals in its public API.

#### Scenario: Crate compiles as workspace member

- GIVEN the anvil workspace
- WHEN `cargo build -p anvil-sdk` is run
- THEN it MUST compile without errors
- AND `anvil-core` MUST NOT appear in the public re-exports of `anvil-sdk`

### Requirement: Anvil Struct

The SDK MUST provide a `Anvil` struct constructed via `Anvil::new() -> Result<Self, AnvilError>`. The constructor MUST NOT require configuration arguments (defaults are sufficient).

#### Scenario: Create Anvil instance

- GIVEN no prior Anvil state
- WHEN `Anvil::new()` is called
- THEN it MUST return `Ok(Anvil)` without errors

### Requirement: Method Surface

The `Anvil` struct MUST implement these methods, all returning `Result<T, AnvilError>`:

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

- GIVEN a `Anvil` instance
- WHEN `sync()` is called
- THEN it MUST return a `SyncReport` with success/failure details

#### Scenario: Query context

- GIVEN a `Anvil` instance
- WHEN `context(ContextFormat::Json)` is called
- THEN it MUST return structured `ContextData`

#### Scenario: Manage secrets

- GIVEN a `Anvil` instance
- WHEN `secret_set("TOKEN", "abc123")` and `secret_get("TOKEN")` are called
- THEN `secret_get` MUST return `Some("abc123")`
- AND `secret_remove("TOKEN")` MUST make `secret_get("TOKEN")` return `None`

### Requirement: Async Support

The SDK SHOULD provide both sync and async (`async fn`) versions of all methods via a feature flag `async`.

#### Scenario: Async compile with feature

- GIVEN a project depending on `anvil-sdk` with feature `async` enabled
- WHEN `cargo check` is run
- THEN async methods MUST be accessible and tokio-compatible

### Requirement: Error Handling

`AnvilError` MUST implement `std::fmt::Display` and `std::error::Error`. It SHOULD wrap a human-readable message string.

#### Scenario: Error propagation

- GIVEN an invalid method call on `Anvil`
- WHEN the call returns `Err(e)`
- THEN `e.to_string()` MUST produce a non-empty string
- AND `e` MUST satisfy `std::error::Error` trait bounds
