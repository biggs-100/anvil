# Proposal: Forge Observability & Telemetry

## Intent
Provide deep local introspection of sandbox operations by capturing events into an Operation Journal, implementing CLI inspection commands, freezing the public API facade, and documenting key architecture choices through ADRs.

## Scope

### In Scope
- Persist `EventBus` events asynchronously to `.forge/journal.jsonl` (NDJSON format).
- Implement commands: `forge history`, `forge explain <runtime>`, `forge trace <operation_id>`, and `forge events [--live]`.
- Design stable API facade `crates/forge-core/src/api/v1.rs` (public `Engine` struct) and route CLI executions through it.
- Write 6 Architecture Decision Records (ADR-0001 to ADR-0006) in `docs/adr/`.

### Out of Scope
- External telemetry/logging platforms (Datadog, OpenTelemetry collector integration).
- IPC mechanisms (Sockets/Named Pipes) for live tailing (uses file polling/watching).

## Capabilities

### New Capabilities
- `observability-journal`: Capturing and persisting `EventBus` events in `.forge/journal.jsonl`.
- `observability-api-v1`: Programmatic stable `Engine` public facade.
- `observability-introspection`: Subcommands `history`, `explain`, `trace`, and `events` for diagnostics.
- `observability-adr`: Complete set of 6 Architecture Decision Records under `docs/adr/`.

### Modified Capabilities
- `event-bus`: Hooks/subscribers to forward memory event broadcasts to the filesystem writer.

## Approach
- **Journaling**: Spawn a background Tokio task in the EventBus to asynchronously write serialized events to `.forge/journal.jsonl`.
- **CLI Commands**: Scan and parse the journal file to build history summaries, explain configuration and cache layouts, reconstruct formatted trace trees, and watch/tail the journal for live events.
- **Engine Facade**: Define standard interfaces in `crates/forge-core/src/api/v1.rs` for callers, migrating CLI commands to use this single wrapper.
- **ADRs**: Document decisions using a unified markdown template in `docs/adr/`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/event_bus.rs` | Modified | Add async journal writing task |
| `crates/forge-core/src/api/v1.rs` | New | High-level `Engine` API facade |
| `crates/forge-core/src/lib.rs` | Modified | Re-export new `api::v1` module |
| `crates/forge-cli/src/main.rs` | Modified | Integrate introspection subcommands and route via Engine |
| `docs/adr/` | New | Write ADR-0001 through ADR-0006 |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Disk write bottleneck | Low | Use buffered channels and asynchronous background writers |
| Journal file growth | Low | Introduce cleanup limits during `forge clean` |
| Interleaved writes | Low | Utilize file locks or single background worker writer |

## Rollback Plan
- Revert commits modifying `crates/forge-core/` and `crates/forge-cli/`.
- Delete generated ADR markdown files in `docs/adr/` and clean `.forge/journal.jsonl`.

## Dependencies
- None.

## Success Criteria
- [ ] Subcommands history, explain, trace, and events output correct data formats.
- [ ] API integrations successfully invoke operations via the `Engine` facade.
- [ ] 6 ADR files are complete and checked in under `docs/adr/`.
