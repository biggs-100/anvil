# Proposal: forge-diagnostic-engine

## Intent
Implement Phase 7 (Diagnostic Engine) to build a sovereign diagnostics platform for Forge sandboxes. Replaces sequential CLI validation logic with a parallelized DAG-based engine executing 11 checks concurrently, computing a health score, and generating repair plans via a decoupled RepairPlanner.

## Scope

### In Scope
- Define `HealthCheck` trait and `DiagnosticEngine` in `crates/forge-core/src/diagnostics/mod.rs`.
- Implement 11 diagnostic checks (Manifest, Lock, Runtime, Secret, Environment, Path, Shim, Hash, Cache, Provider, Profile).
- Model structured `Finding` with codes (FG001–FG011), severity grades, confidence intervals, and explanations.
- Build DAG runner with `tokio` that short-circuits downstream dependents on critical blocker failures.
- Implement `DiagnosticMode` (Fast vs Deep) to optimize run speed.
- Implement `HealthScore` (0-100 scale, capped at 40 max if critical findings exist).
- Map findings to `RepairPlan` via decoupled `RepairPlanner` with `QuickFixAction` enum.
- Add CLI commands `forge doctor [--deep] [--json]` and `forge ai doctor` (masking all secrets).

### Out of Scope
- Writing actual execution blocks for external repair scripts (only planning/returning actions is in scope).

## Capabilities

### New Capabilities
- `diagnostic-engine`: Main orchestrator and tokio DAG runner.
- `diagnostic-checks`: Collection of 11 `HealthCheck` providers.
- `diagnostic-repair-planner`: Decoupled parser converting report findings into QuickFix plans.
- `diagnostic-cli-commands`: Commands `doctor` and `ai doctor`.

### Modified Capabilities
- `config-validation`: Adapt validation routines to return `Finding` models.

## Approach
- Utilize `tokio` for spawning parallel diagnostic checks defined as a DAG.
- Mask all environment variable payloads in `Finding` to guarantee zero plaintext leaks.
- Decouple report generation from repair execution using `QuickFixAction`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/diagnostics/mod.rs` | New | Module for traits, engine, and 11 checks. |
| `crates/forge-core/src/lib.rs` | Modified | Export diagnostics module. |
| `crates/forge-core/src/operations/mod.rs` | Modified | Add `RepairPlanner` mapping to `RepairPlan`. |
| `crates/forge-cli/src/main.rs` | Modified | Route `forge doctor [--deep] [--json]` and `forge ai doctor`. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Concurrency Overload | Low | Restrict parallel check execution using DAG dependencies and disk-bound execution queues. |
| Secret Leakage | Low | Mandate boolean presence or masked payloads (`[MASKED]`) in all env/secret diagnostics. |

## Rollback Plan
- Revert code changes to `crates/forge-cli` and `crates/forge-core` using git. No persistent database state is modified.

## Dependencies
- `tokio` (concurrency orchestration).
- `serde` / `serde_json` (structured reporting).

## Success Criteria
- [ ] Concurrently runs 11 checks and computes correct HealthScore.
- [ ] Downstream checks abort/short-circuit when upstream blocker checks fail.
- [ ] `forge doctor --json` prints valid JSON with zero plaintext secret leakage.
- [ ] `RepairPlanner` maps report findings into `RepairPlan` actions correctly.
