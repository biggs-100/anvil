Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Anvil Diagnostic Engine

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 700-900 lines of Rust |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Diagnostic Engine & Models | PR 1 | Base framework & tokio DAG scheduler |
| 2 | 11 Concrete Health Checks | PR 2 | Implement checks (FG001-FG011) |
| 3 | Repair Planner, CLI & AI doctor | PR 3 | End-to-end repair, custom serializer |

## Phase 1: Diagnostic Concurrency Engine & Models (PR 1)

- [x] 1.1 Create `crates/anvil-core/src/diagnostics/mod.rs` defining models: `HealthCheck` trait, `Finding`, `Severity`, `Explanation`, `QuickFixAction`, and `DiagnosticReport`.
- [x] 1.2 Implement tokio-based concurrent DAG scheduler resolving check dependencies and short-circuiting downstream checks if upstream fails with `CRITICAL`.
- [x] 1.3 Export `diagnostics` module in `crates/anvil-core/src/lib.rs`.
- [x] 1.4 Write unit tests in `crates/anvil-core/src/diagnostics/mod.rs` verifying HealthScore math rules and short-circuiting execution flow.

## Phase 2: 11 Concrete Health Checks (PR 2)

- [x] 2.1 Implement 11 checks: `ManifestCheck`, `LockCheck`, `RuntimeCheck`, `HashCheck`, `SecretCheck`, `EnvironmentCheck`, `PathCheck`, `ShimCheck`, `CacheCheck`, `ProviderCheck`, and `ProfileCheck` in `crates/anvil-core/src/diagnostics/mod.rs`.
- [x] 2.2 Configure checks to abort downstream dependants (e.g., lock, runtime, env checks depend on `ManifestCheck`).
- [x] 2.3 Write unit tests in `crates/anvil-core/src/diagnostics/mod.rs` verifying check operations against mocked files.

## Phase 3: Repair Planner, CLI & AI Doctor (PR 3)

- [x] 3.1 Implement `RepairPlanner` converting findings to `RepairPlan` in `crates/anvil-core/src/operations/mod.rs`.
- [x] 3.2 Implement a custom `serde::Serialize` wrapper in `crates/anvil-core/src/diagnostics/mod.rs` to dynamically mask sensitive environment values (e.g., `SetEnvVar`) with `[MASKED]`.
- [x] 3.3 Map CLI subcommands `doctor` and `ai doctor` in `crates/anvil-cli/src/main.rs`.
- [x] 3.4 Write integration tests in `crates/anvil-cli/tests/` to verify CLI outputs and AI doctor JSON schema compliance.
