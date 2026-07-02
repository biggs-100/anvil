# Proposal: forge-snapshot

## Intent

Save and restore full environment state for rollback before risky operations, bug reproduction across runs, and safe runtime version experimentation.

## Scope

### In Scope
- `forge snapshot` — save state to `.forge/snapshots/{timestamp}/`
- `forge snapshot list` — list available snapshots with metadata
- `forge restore snapshot <name>` — restore environment to saved state
- Named snapshots via `--name` flag (human-readable aliases)
- Capture: forge.toml, forge.lock, state.json, journal.jsonl (last 100), snapshot.json
- No runtime binaries in snapshots (descriptors only)
- Runtimes re-synced to match locked versions on restore

### Out of Scope
- Cloud sync or distributed snapshot sharing
- Runtime binaries included in snapshots
- Cross-machine restore
- Automatic or scheduled snapshots

## Capabilities

### New Capabilities
- `forge-snapshot`: Snapshot save, list, and restore CLI commands for environment state

### Modified Capabilities
- `cli-commands-lifecycle`: New CLI commands (`forge snapshot`, `forge snapshot list`, `forge restore snapshot`)
- `environment-lifecycle-rfc`: Define snapshot/restore lifecycle transitions and valid state preconditions

## Approach

Directory-based snapshots under `.forge/snapshots/{timestamp}/`. Snapshot captures config + lock + lifecycle state + journal as flat files. Restore replaces config/lock files, then re-syncs runtimes to match locked versions via the existing sync pipeline. No runtime binaries stored — only descriptors.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-cli/` | New | snapshot and restore CLI subcommands |
| `crates/forge-core/` | New | Snapshot engine: capture, list, restore logic |
| `.forge/snapshots/` | New | Snapshot storage directory |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Disk space from accumulated snapshots | Medium | Manual cleanup; `forge snapshot gc` deferred |

## Rollback Plan

Restore from a previous snapshot via `forge restore snapshot <name>`. Snapshots are immutable after creation, so undoing a mistaken restore is always possible: re-run restore with the previous snapshot name.

## Dependencies

- Existing Runtime Engine sync pipeline (reused by restore)
- Config engine (forge.toml read/write)
- Observability journal (captures event history in snapshots)

## Success Criteria

- [ ] `forge snapshot` creates `.forge/snapshots/{timestamp}/` with all expected files
- [ ] `forge snapshot list` shows all snapshots with metadata (timestamp, name, runtime count)
- [ ] `forge restore snapshot <name>` replaces config/lock and re-syncs runtimes
- [ ] Named snapshots appear correctly in list output
