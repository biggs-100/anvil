Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Forge Observability & Telemetry

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 500-700 lines + 6 ADRs |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Stable API Facade & Journal Logging | PR 1 | Facade in `api/v1.rs` & EventBus async logger |
| 2 | Architecture Decision Records (ADRs) | PR 2 | Write ADR markdown files ADR-0001 to ADR-0006 |
| 3 | CLI Introspection Commands | PR 3 | Implement commands & remap CLI main |

## Phase 1: API Facade & Journal Logging (PR 1)

- [x] 1.1 Create `crates/forge-core/src/api/v1.rs` exposing the `Engine` struct, v1 types, and unified public methods.
- [x] 1.2 Modify `crates/forge-core/src/lib.rs` to re-export the `api::v1` module.
- [x] 1.3 Update `crates/forge-core/src/event_bus.rs` to spawn a background Tokio task on EventBus creation that asynchronously writes events to `.forge/journal.jsonl`.
- [x] 1.4 Write unit tests verifying serialization of events and concurrent logging safety to `.forge/journal.jsonl`.

## Phase 2: Architecture Decision Records (PR 2)

- [x] 2.1 Create `docs/adr/` directory.
- [x] 2.2 Write ADR-0001 through ADR-0006 under `docs/adr/` following standard Status/Context/Decision/Consequences formats.

## Phase 3: CLI Introspection Commands (PR 3)

- [x] 3.1 Implement subcommands `history`, `explain`, `trace`, and `events` in `crates/forge-cli/src/main.rs`.
- [x] 3.2 Remap CLI command handlers to exclusively call the `Engine` API facade.
- [x] 3.3 Add CLI integration tests checking command outputs and live tailing (`--live`) behaviour.
