# Diagnostic Engine Specification

## Purpose
Define the orchestrator, DAG-based concurrent execution model, short-circuiting logic, and health score computation rules for the diagnostics platform (RFC-0013).

## Requirements

### Requirement: Asynchronous DAG Task Execution
The `DiagnosticEngine` MUST schedule and execute registered checks concurrently according to their dependency graph using `tokio`.

| Feature | Behavior |
|---|---|
| Concurrency | Executed in parallel using `tokio::spawn` |
| Execution Order | Dependency-first resolution based on check DAG |
| Registration | Engine supports dynamic check registration via a `HealthCheck` trait |

#### Scenario: Concurrently Running Independent Checks
- GIVEN checks `PathCheck` and `SecretCheck` with no declared dependencies
- WHEN `DiagnosticEngine::run` is invoked
- THEN both checks MUST execute concurrently

---

### Requirement: Dependency Short-Circuiting
The engine MUST abort downstream checks and mark them as skipped if an upstream dependency yields a `CRITICAL` or `ERROR` finding.

| Upstream Failure | Skipped Downstream Checks |
|---|---|
| Manifest (FG001, FG002) | Lock, Environment, Profile, Runtime, Shim, Hash |
| Lock (FG003) | Runtime, Shim, Hash |
| Runtime (FG005) | Hash |

#### Scenario: Short-circuit Downstream on Blocker
- GIVEN `ManifestCheck` fails with a `CRITICAL` missing manifest finding
- WHEN the DAG runs `LockCheck` (which depends on `manifest`)
- THEN the engine MUST skip `LockCheck` and record a skip trace instead of running it

---

### Requirement: HealthScore Computation
The engine MUST compute an integer HealthScore between `0` and `100` based on finding severities.

| Finding Severity | Points Deducted | Score Guards |
|---|---|---|
| `CRITICAL` | 30 points | Capped at 40 maximum if any `CRITICAL` finding is present |
| `ERROR` | 15 points | Minimum score is 0 |
| `WARNING` | 5 points | Maximum score is 100 |
| `INFO` | 0 points | |

#### Scenario: HealthScore Capping with Critical Finding
- GIVEN a report with one `CRITICAL` finding and one `WARNING` finding
- WHEN the engine calculates the score (100 - 30 - 5 = 65)
- THEN the calculated HealthScore MUST be capped at 40

---

### RFC-0013 Diagnostic Platform Specification Reference
The engine conforms to the RFC-0013 interface contracts for `HealthCheck` traits, `DiagnosticContext`, `DiagnosticMode`, `Finding`, and `DiagnosticReport` formats.
