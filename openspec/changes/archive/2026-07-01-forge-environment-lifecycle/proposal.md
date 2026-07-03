# Proposal: Anvil Environment Lifecycle

## Intent
Implement Environment Lifecycle (RFC-0011) and modular Operations Layer. Introduces Plan Engine, Event Bus progress dispatch, and Staging-to-Commit atomic transactional installations across the 13 CLI environment commands.

## Scope

### In Scope
- Create RFC-0011 detailing 10 states (UNINITIALIZED, INITIALIZED, RESOLVED, LOCKED, SYNCED, READY, ACTIVE, DIRTY, OUTDATED, BROKEN) and transitions.
- Modular Operations Layer: `trait Operation` returning `OperationResult` (status, duration, warnings, changes, diagnostics).
- Plan Engine: deterministic planner (`SyncPlan`, `RepairPlan`) before mutations.
- Atomic Transactional Installations: download/extract to `.anvil/staging`, promote/commit on validation success, rollback/discard on failure.
- Tokio-based Event Bus (`tokio::sync::broadcast`) for progress dispatch.
- Map 13 CLI commands: `init`, `resolve`, `lock`, `sync`, `up`, `run`, `shell`, `clean`, `gc`, `status`, `inspect`, `repair`, `plan`.
- Implement `RepairOperation` (5-step pipeline) and `ValidateOperation` (read-only health checks).
- Enforce idempotency on operations (sync, repair, clean, gc).

### Out of Scope
- Integrating cloud remote caches (deferred).
- Multi-user OS keychain integration (deferred).

## Capabilities

### New Capabilities
- `environment-lifecycle-rfc`: RFC-0011 state-machine design.
- `operations-layer`: Operation trait, OperationResult, and CLI/core command implementations.
- `plan-engine`: SyncPlan/RepairPlan calculator.
- `event-bus`: Structured broadcast progress dispatcher.
- `atomic-transactions`: Staging folder commit/rollback transaction engine.
- `cli-commands-lifecycle`: Command-line interface for the 13 lifecycle commands.

### Modified Capabilities
- `runtime-manager`: Modified to support staging folders and commit phase promotions.

## Approach
- Rust `Operation` trait architecture for core logic separation.
- `tokio::sync::broadcast` for decoupled asynchronous progress telemetry.
- Transactional filesystem staging under `.anvil/staging` with fallback/atomic directory rename promotion.
- CLI commands re-mapped to trigger plans, operations, and format event feeds.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/anvil-core/src/types.rs` | Modified | Add `LifecycleState`, `Event`, `OperationResult`. |
| `crates/anvil-core/src/operations/` | New | Implement `Operation` traits, planner, and engine. |
| `crates/anvil-cli/src/main.rs` | Modified | Map 13 CLI commands to operations and Event Bus. |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Non-atomic directory renames across partitions | Medium | Force staging directory to exist on same filesystem partition as cache. |
| File locks on Windows blocking promotion | High | Implement retry policies, error handling, and file-lock checks during rename. |

## Rollback Plan
- Promotion failures: delete `.anvil/staging` directory and restore from `.backup`.
- Deployment revert: git revert to restore previous CLI code and run `anvil clean --all` to reset cache.

## Dependencies
- Tokio broadcast channel support (Rust tokio crate).

## Success Criteria
- [ ] 100% of toolchain downloads are atomic (zero partial installations on disk).
- [ ] Test coverage verifies all 10 states and transitions of RFC-0011.
- [ ] Idempotent execution verified for sync, repair, clean, and gc.
