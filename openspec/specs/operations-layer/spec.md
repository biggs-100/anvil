# Operations Layer Specification

## Purpose

Define the unified Operations Layer schema, specifically `trait Operation` and `OperationResult`, ensuring a consistent interface and output format for all environment operations.

## Requirements

### Requirement: Unified Operation Trait
All mutation and read-only tasks MUST implement a standard `Operation` interface providing planning, validation, and execution capabilities.

- `Operation::name(&self) -> &str`: Returns the identifier of the operation.
- `Operation::plan(&self, context: &Context) -> Result<Box<dyn Plan>, Error>`: Computes execution plan without modifying filesystem.
- `Operation::execute(&self, context: &mut Context, plan: Box<dyn Plan>) -> Result<OperationResult, Error>`: Executes planned mutations.

### Requirement: Standard Operation Result Schema
Every execution of an `Operation` MUST produce a standard `OperationResult` structured document:

| Field | Type | Description |
|---|---|---|
| `status` | Enum | Result status: `SUCCESS`, `FAILURE`, `WARNING`, or `SKIPPED`. |
| `duration_ms` | u64 | Total execution time in milliseconds. |
| `warnings` | Array of Strings | Non-fatal issues encountered. |
| `changes` | Array of Change Objects | Record of filesystem/system mutations (e.g. `added`, `deleted`, `modified`). |
| `diagnostics` | Array of Diagnostic Objects | Execution telemetry, trace data, or error details on failure. |

#### Scenario: Successful Operation Execution
- GIVEN a valid `SyncPlan`
- WHEN `SyncOperation::execute` is called
- THEN the system MUST return an `OperationResult` with `status` set to `SUCCESS`, populate the list of filesystem `changes`, and specify a non-zero `duration_ms`.

#### Scenario: Dry-run Planning
- GIVEN a request to perform an operation dry-run
- WHEN `Operation::plan` is called
- THEN the system MUST return the computed `Plan` without modifying the filesystem or invoking `execute`.

#### Scenario: Execution Failure with Diagnostics
- GIVEN an operation that encounters a network timeout during toolchain download
- WHEN `Operation::execute` is called
- THEN the system MUST return an `OperationResult` with `status` set to `FAILURE` containing error diagnostics and any partial warnings.

---

### Requirement: Plugin-Registered Operations

The Engine MUST accept `Operation` implementations registered via `PluginRegistry`. Plugin operations MUST implement the same `Operation` trait (`name`, `plan`, `execute`) and MUST be dispatched through the same `Operation` interface as built-in operations.

(Previously: Only built-in operations were available. Plugin operations extend the engine with third-party mutation/query capabilities.)

#### Scenario: Plugin Operation Is Dispatched
- GIVEN a plugin registers an `Operation` with `name = "deploy"`
- WHEN the Engine dispatches `Operation::execute("deploy", context, plan)`
- THEN the engine MUST route to the plugin's `execute` implementation and return a standard `OperationResult`

#### Scenario: Plugin Operation Plan Is Invoked
- GIVEN a plugin registers an `Operation` with `name = "deploy"`
- WHEN `Operation::plan("deploy", context)` is called
- THEN the engine MUST invoke the plugin's `plan` method, which MUST return a `Plan` without mutating the filesystem
