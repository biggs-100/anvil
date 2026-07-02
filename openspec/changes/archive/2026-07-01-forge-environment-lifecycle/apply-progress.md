# Implementation Progress: forge-environment-lifecycle

**Change**: forge-environment-lifecycle
**Mode**: openspec
**Resolved Workload Strategy**: `size:exception` (Single large PR)

## Completed Tasks
- [x] **Task 1.1**: Define `LifecycleState` (10 states), `OperationResult`, `ChangeRecord`, `Event`, `EventStatus`, and `OperationStatus` in `crates/forge-core/src/types.rs`.
- [x] **Task 1.2**: Create lightweight `EventBus` using `tokio::sync::broadcast` in `crates/forge-core/src/event_bus.rs`.
- [x] **Task 1.3**: Create traits `Plan` and `Operation` in `crates/forge-core/src/operations/mod.rs`.
- [x] **Task 1.4**: Add unit tests in `types.rs` checking transitions: `LOCKED` -> `READY` -> `ACTIVE`, and recovery transitions.
- [x] **Task 2.1**: Update `installer.rs` to staging directory extraction under `.forge/staging/<operation_id>/`.
- [x] **Task 2.2**: Implement directory promotion via `std::fs::rename`, backup under `.forge/backup/`, and rollback on failure.
- [x] **Task 2.3**: Update `cache.rs` to decouple shims cache regeneration to run only on successful commit.
- [x] **Task 2.4**: Add unit tests in `installer.rs` simulating staging, empty zip checks, hash verification, and transactional rollback.
- [x] **Task 3.1**: Implement the 11 operations (Init, Resolve, Lock, Sync, Gc, Clean, Run, Shell, Repair, Plan, Validate) in `crates/forge-core/src/operations/mod.rs` as async operations.
- [x] **Task 3.2**: Remap the 13 CLI command handlers in `crates/forge-cli/src/main.rs` to use the operations layer and subscribe to the EventBus progress telemetry.
- [x] **Task 3.3**: Add integration tests in `crates/forge-core/tests/integration.rs` verifying idempotency (syncing twice returns `Skipped` status) and E2E lifecycle state transitions.

## Created/Modified Files
| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/types.rs` | Modified | Added lifecycle state machine enums/structs & transition check method + tests. |
| `crates/forge-core/src/event_bus.rs` | Created | Implemented tokio broadcast based telemetry event bus. |
| `crates/forge-core/src/lib.rs` | Modified | Registered modules and exported new public API traits & structures. |
| `crates/forge-core/src/installer.rs` | Modified | Rewrote runtime installation to use staging, backup, atomic rename, and automatic rollback + offline `file://` copies and tests. |
| `crates/forge-core/src/operations/mod.rs` | Created | Built the operations layer executing via `async fn execute`. |
| `crates/forge-cli/src/main.rs` | Modified | Mapped CLI commands to operations layer, subscribed progress telemetry, and added status check / save. |
| `crates/forge-core/Cargo.toml` | Modified | Added `serde_json` to dependencies. |
| `crates/forge-core/tests/integration.rs` | Modified | Appended E2E transition validation and idempotency checks. |

## Deviations or Issues
None. All 23 tests run and pass cleanly.
