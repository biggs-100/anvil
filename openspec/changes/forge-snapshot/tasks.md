# Tasks: forge-snapshot

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~200-350 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Snapshot Module

- [x] 1.1 Create `crates/forge-core/src/snapshot.rs` with `SnapshotMetadata` struct (Serialize/Deserialize) and `SnapshotManager` struct holding workspace_root, snapshots_dir
- [x] 1.2 Implement `SnapshotManager::create()` — copy forge.toml/forge.lock verbatim, capture state.json via `compute_current_state()`, read last 100 lines of journal.jsonl, write snapshot.json
- [x] 1.3 Modify `crates/forge-core/src/lib.rs` — add `pub mod snapshot;` and re-export `SnapshotManager`, `SnapshotMetadata`

## Phase 2: List + Restore

- [x] 2.1 Implement `SnapshotManager::list()` — read `.forge/snapshots/` dirs, deserialize each snapshot.json, sort by created_at desc
- [x] 2.2 Implement `SnapshotManager::restore()` — backup current forge.toml/forge.lock to `.bak` files, copy snapshot files to workspace, then CLI dispatches `forge up` (LockOperation + engine.sync)
- [x] 2.3 Add dry-run mode — display what would restore without modifying files
- [x] 2.4 Handle errors: missing forge.toml, non-existent snapshot, failed forge up after restore (bak rollback)

## Phase 3: CLI

- [x] 3.1 Add `"snapshot"` to `BUILTIN_COMMANDS` array in `crates/forge-cli/src/main.rs`
- [x] 3.2 Add `Snapshot` subcommand (create) and nested `SnapshotCommands::{Create, List, SnapshotRestore}` variants; wire snapshot/restore dispatch in `run_cli()` with SnapshotManager calls and output
- [x] 3.3 Dispatch all 3 commands to snapshot module; add forge up after restore with bak rollback on failure

## Phase 4: Testing

- [x] 4.1 Test create — temp dir with forge.toml/forge.lock/state → verify all 5 snapshot files and byte-identical copies
- [x] 4.2 Test list — create 3 snapshots → verify correct metadata
- [x] 4.3 Test restore dry-run — verify no files changed and preview displayed
- [x] 4.4 Test restore missing snapshot — verify error returned and no files modified
- [x] 4.5 Test missing forge.toml — auto-name format, metadata content, bak files created
