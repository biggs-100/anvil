# Delta for Operations Layer

## ADDED Requirements

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
