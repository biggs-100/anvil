# Implementation Progress: Forge Observability & Telemetry

**Change**: forge-observability-telemetry
**Mode**: Standard (OpenSpec)
**Workload Mode**: `size:exception`

All 9 tasks defined in `tasks.md` have been successfully implemented and verified with zero compiler warnings and 100% test success.

## Completed Tasks
- [x] 1.1 Create `crates/forge-core/src/api/v1.rs` exposing the `Engine` struct, v1 types, and unified public methods.
- [x] 1.2 Modify `crates/forge-core/src/lib.rs` to re-export the `api::v1` module.
- [x] 1.3 Update `crates/forge-core/src/event_bus.rs` to spawn a background Tokio task on EventBus creation that asynchronously writes events to `.forge/journal.jsonl`.
- [x] 1.4 Write unit tests verifying serialization of events and concurrent logging safety to `.forge/journal.jsonl`.
- [x] 2.1 Create `docs/adr/` directory.
- [x] 2.2 Write ADR-0001 through ADR-0006 under `docs/adr/` following standard Status/Context/Decision/Consequences formats.
- [x] 3.1 Implement subcommands `history`, `explain`, `trace`, and `events` in `crates/forge-cli/src/main.rs`.
- [x] 3.2 Remap CLI command handlers to exclusively call the `Engine` API facade.
- [x] 3.3 Add CLI integration tests checking command outputs and live tailing (`--live`) behaviour.

## Files Created / Modified

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/api/v1.rs` | Created | Stable API facade exposing `Engine` with `sync`, `repair`, `clean`, `explain`, `history`, `trace`, and `events` APIs. |
| `crates/forge-core/src/api/mod.rs` | Created | Export sub-module `v1`. |
| `crates/forge-core/src/lib.rs` | Modified | Re-exported stable `api::v1` namespace and types. |
| `crates/forge-core/src/event_bus.rs` | Modified | Spawns background tokio task for NDJSON `.forge/journal.jsonl` writes; added safe `new_with_journal` and unit tests. |
| `crates/forge-cli/src/main.rs` | Modified | Added subcommands `history`, `explain`, `trace`, `events`. Remapped other execution handlers to `Engine`. Added table and ASCII tree formatting. |
| `docs/adr/ADR-0001.md` | Created | Architectural Decision Record for Asynchronous Journal Storage. |
| `docs/adr/ADR-0002.md` | Created | Architectural Decision Record for Engine Facade Isolation. |
| `docs/adr/ADR-0003.md` | Created | Architectural Decision Record for In-process EventBus Hook. |
| `docs/adr/ADR-0004.md` | Created | Architectural Decision Record for CLI Introspection Interface. |
| `docs/adr/ADR-0005.md` | Created | Architectural Decision Record for Local JSON Lines Format. |
| `docs/adr/ADR-0006.md` | Created | Architectural Decision Record for Cache Integrity & Verification. |
| `crates/forge-core/tests/integration.rs` | Modified | Added integration tests verifying `explain` resolution, `trace` ASCII formatting, and `events` live tailing. |
| `openspec/changes/forge-observability-telemetry/tasks.md` | Modified | Marked all 9 tasks as complete (`[x]`). |

## Deviations or Issues
None. All components function strictly according to design and specifications.
