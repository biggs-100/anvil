# Design: forge-snapshot

## Technical Approach

New `SnapshotManager` in `anvil-core` handles snapshot CRUD as directory-based flat files under `.anvil/snapshots/{name}/`. The CLI adds three commands via `clap` subcommands. Restore delegates to the existing `anvil up` pipeline for runtime sync. No new runtime binaries or caches are stored — only descriptors.

## Architecture Decisions

### Decision: Snapshot name format

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Timestamp (ISO 8601) | Sortable, collision-free, self-documenting | **Chosen** — `YYYY-MM-DDTHH-MM-SS` (UTC) |
| Sequential (v1, v2…) | Meaningless without context, collision under concurrency | Rejected |

### Decision: Journal capture strategy

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Read last N lines from journal.jsonl | Simple, no schema change, handles arbitrary line counts | **Chosen** — read last 100 lines |
| Structured query via EventBus | Over-engineered for an append-only log | Rejected |

### Decision: Backup location for restore safety

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `.anvil/snapshots/.backup/{ts}/` | Co-located with snapshots, self-cleaning on `anvil clean` | **Chosen** |
| User home temp dir | Cross-device confusion, invisible to anvil cleanup | Rejected |

### Decision: Restore triggers anvil up (not resolve+sync)

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Delegate to `anvil up` | Reuses existing lock + sync pipeline, consistent UX | **Chosen** |
| Inline resolve+sync | Duplicates pipeline logic, diverges from UX | Rejected |

## Data Flow

```
Snapshot Create:
  workspace_root
     │
     ├── anvil.toml ──────────────────────┐
     ├── anvil.lock ──────────────────────┤
     ├── compute_current_state() ─────────┤
     ├── .anvil/journal.jsonl (last 100)──┤
     └── metadata (name, desc, version) ──┤
                                          ▼
                              .anvil/snapshots/{name}/
                              ├── anvil.toml (copy)
                              ├── anvil.lock (copy)
                              ├── state.json
                              ├── journal.jsonl
                              └── snapshot.json

Snapshot Restore:
  .anvil/snapshots/{name}/
     │
     ├── anvil.toml ──→ backup current → copy to workspace
     ├── anvil.lock ──→ backup current → copy to workspace
     │
     └── anvil up (LockOperation + engine.sync())
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/src/snapshot.rs` | Create | `SnapshotManager` with `create()`, `list()`, `restore()` |
| `crates/anvil-core/src/lib.rs` | Modify | Add `pub mod snapshot;` and re-exports |
| `crates/anvil-cli/src/main.rs` | Modify | Add `Snapshot`, `SnapshotList`, `Restore` subcommands + `"snapshot"` to `BUILTIN_COMMANDS` |

## Interfaces / Contracts

```rust
// crates/anvil-core/src/snapshot.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub name: String,
    pub created_at: String,          // ISO 8601 UTC
    pub forge_version: String,
    pub runtime_count: usize,
    pub description: Option<String>,
}

pub struct SnapshotManager {
    workspace_root: PathBuf,
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new(workspace_root: &Path) -> Self;

    pub fn create(&self, name: Option<&str>, description: Option<&str>) -> Result<String, String>;

    pub fn list(&self) -> Result<Vec<SnapshotMetadata>, String>;

    pub fn restore(&self, name: &str, dry_run: bool) -> Result<(), String>;
}
```

Key signatures reused from existing codebase:
- `compute_current_state(&Path, &Path) -> LifecycleState` (crate::state)
- `load_lockfile(&Path) -> Result<Lockfile, String>` (crate::lock)
- `load_config(&Path) -> Result<ForgeConfig, String>` (crate::manifest)

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | SnapshotManager::create | Temp dir with anvil.toml/anvil.lock → verify all 5 files created |
| Unit | SnapshotManager::list | Create 3 snapshots → verify sorted output + metadata |
| Unit | SnapshotManager::restore dry-run | Verify no files changed, output preview |
| Unit | SnapshotManager::restore missing | Verify error + no files modified |
| Integration | CLI `anvil snapshot` dispatch | Match existing CLI test pattern (temp workspace, exec anvil CLI) |
| Integration | Restore → anvil up chain | Temp workspace → create snapshot → modify lock → restore → verify lock reverted |

## Migration / Rollout

No migration required. Snapshots are opt-in — existing `.anvil/` directories without `snapshots/` work normally. `anvil clean` removes `.anvil/snapshots/` as part of the existing clean scope.

## Open Questions

None.
