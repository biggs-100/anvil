# Design: forge-snapshot

## Technical Approach

New `SnapshotManager` in `forge-core` handles snapshot CRUD as directory-based flat files under `.forge/snapshots/{name}/`. The CLI adds three commands via `clap` subcommands. Restore delegates to the existing `forge up` pipeline for runtime sync. No new runtime binaries or caches are stored — only descriptors.

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
| `.forge/snapshots/.backup/{ts}/` | Co-located with snapshots, self-cleaning on `forge clean` | **Chosen** |
| User home temp dir | Cross-device confusion, invisible to forge cleanup | Rejected |

### Decision: Restore triggers forge up (not resolve+sync)

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Delegate to `forge up` | Reuses existing lock + sync pipeline, consistent UX | **Chosen** |
| Inline resolve+sync | Duplicates pipeline logic, diverges from UX | Rejected |

## Data Flow

```
Snapshot Create:
  workspace_root
     │
     ├── forge.toml ──────────────────────┐
     ├── forge.lock ──────────────────────┤
     ├── compute_current_state() ─────────┤
     ├── .forge/journal.jsonl (last 100)──┤
     └── metadata (name, desc, version) ──┤
                                          ▼
                              .forge/snapshots/{name}/
                              ├── forge.toml (copy)
                              ├── forge.lock (copy)
                              ├── state.json
                              ├── journal.jsonl
                              └── snapshot.json

Snapshot Restore:
  .forge/snapshots/{name}/
     │
     ├── forge.toml ──→ backup current → copy to workspace
     ├── forge.lock ──→ backup current → copy to workspace
     │
     └── forge up (LockOperation + engine.sync())
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/snapshot.rs` | Create | `SnapshotManager` with `create()`, `list()`, `restore()` |
| `crates/forge-core/src/lib.rs` | Modify | Add `pub mod snapshot;` and re-exports |
| `crates/forge-cli/src/main.rs` | Modify | Add `Snapshot`, `SnapshotList`, `Restore` subcommands + `"snapshot"` to `BUILTIN_COMMANDS` |

## Interfaces / Contracts

```rust
// crates/forge-core/src/snapshot.rs

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
| Unit | SnapshotManager::create | Temp dir with forge.toml/forge.lock → verify all 5 files created |
| Unit | SnapshotManager::list | Create 3 snapshots → verify sorted output + metadata |
| Unit | SnapshotManager::restore dry-run | Verify no files changed, output preview |
| Unit | SnapshotManager::restore missing | Verify error + no files modified |
| Integration | CLI `forge snapshot` dispatch | Match existing CLI test pattern (temp workspace, exec forge CLI) |
| Integration | Restore → forge up chain | Temp workspace → create snapshot → modify lock → restore → verify lock reverted |

## Migration / Rollout

No migration required. Snapshots are opt-in — existing `.forge/` directories without `snapshots/` work normally. `forge clean` removes `.forge/snapshots/` as part of the existing clean scope.

## Open Questions

None.
