Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Anvil Environment Lifecycle

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 700-900 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Lifecycle Types, Event Bus, and Plan/Operation Contracts | PR 1 | Base branch; defines Core models and contracts, includes unit tests |
| 2 | Transactional Staging & Commit | PR 2 | Depends on PR 1; staging layout, promotional rename, and rollbacks |
| 3 | Operations Layer & 13 CLI Commands Mapping | PR 3 | Depends on PR 2; implement 10 operations, CLI remapping, integrations |

## Phase 1: Foundation (PR 1)

- [x] 1.1 In `crates/anvil-core/src/types.rs`, define `LifecycleState`, `OperationResult`, `Event`, `EventStatus`, and `OperationStatus`.
- [x] 1.2 Create `crates/anvil-core/src/event_bus.rs` using `tokio::sync::broadcast` for progress telemetry.
- [x] 1.3 Create `crates/anvil-core/src/operations/mod.rs` and define the `Plan` and `Operation` traits.
- [x] 1.4 Write unit tests in `crates/anvil-core/src/types.rs` verifying lifecycle state transitions. Verify by running `cargo test --lib types`.

## Phase 2: Transactional Staging (PR 2)

- [x] 2.1 Update `crates/anvil-core/src/installer.rs` to download and extract to staging directory `.anvil/staging/<operation_id>`.
- [x] 2.2 Implement atomic directory rename promotion, backup creation, and rollback logic on failure in `crates/anvil-core/src/installer.rs`.
- [x] 2.3 Update `crates/anvil-core/src/cache.rs` to decouple shims cache regeneration, executing only after a successful commit.
- [x] 2.4 Write unit tests in `crates/anvil-core/src/installer.rs` simulating rename failure and rollback. Verify by running `cargo test --lib installer`.

## Phase 3: Operations & CLI Integration (PR 3)

- [x] 3.1 Implement the 10 operations (Resolve, Lock, Sync, Gc, Clean, Run, Shell, Repair, Plan, Validate) in `crates/anvil-core/src/operations/mod.rs`.
- [x] 3.2 Remap the 13 CLI command handlers in `crates/anvil-cli/src/main.rs` to use the operations layer and event bus progress display.
- [x] 3.3 Write integration tests in `crates/anvil-core/tests/integration.rs` verifying idempotency and end-to-end flows. Verify by running `cargo test --test integration`.
