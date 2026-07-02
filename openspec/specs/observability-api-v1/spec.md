# Observability API v1 Specification

## Purpose
Define Rust contracts for the stable `Engine` facade in `crates/forge-core/src/api/v1.rs`.

## Requirements

### Requirement: Facade Interface
The `crates/forge-core/src/api/v1.rs` module MUST expose the `Engine` struct as the stable programmatic API.

### Requirement: Command Routing Isolation
The CLI subcommands MUST route all execution, history, tracing, and explanation queries exclusively through the `Engine` public methods.

| Engine Method | Input Params | Return Type | Description |
|---|---|---|---|
| `history` | limit: Option<usize> | Result<Vec<Operation>> | Queries history of operations from journal. |
| `explain` | runtime: &str | Result<RuntimeDetail> | Resolves configuration, cache, shims. |
| `trace` | id: Uuid | Result<TraceTree> | Builds execution hierarchy tree. |
| `events` | live: bool | Result<Receiver<Event>> | Streams events (optionally watching the file). |

#### Scenario: CLI History via Facade
- GIVEN a `forge history` command invocation
- WHEN the CLI executes
- THEN it MUST call `Engine::history()` and format the returned list of operations.
