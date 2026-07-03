# Proposal: forge-bundle

## Intent

Forge resolves, downloads, and syncs runtimes, but has no portable distribution format. Users cannot share a pinned environment without cloning the repo. Add `anvil bundle` / `anvil restore` for a self-verifying `.forge` archive of descriptors — no runtime binaries.

## Scope

### In Scope
- `anvil bundle` — produce `project.forge` from current workspace
- `anvil restore` — recreate environment from `project.forge`
- `.forge` archive format (deterministic internal structure)
- SHA-256 checksum verification
- Context metadata (non-sensitive)
- Explicit secrets exclusion

### Out of Scope
- Runtime binaries in bundle, cloud upload, registry publishing, encryption, delta bundles, diff/merge

## Capabilities

### New Capabilities
- `forge-bundle`: Bundle and restore commands — archive format, checksum verification, metadata extraction

### Modified Capabilities
None — existing specs unchanged.

## Approach

Add `bundle` and `restore` CLI subcommands via clap. Core logic in `anvil-core` under a new `bundle` module:
1. **Bundle**: Read `anvil.toml` + `anvil.lock`, collect metadata, compute SHA-256 checksums, write deterministic tar+gzip archive as `project.forge`.
2. **Restore**: Extract archive, verify checksums, write manifest+lockfile, delegate to `anvil up`.

Use `tar` + `flate2` crates (stable, cross-platform). Sort entries by filename for deterministic output.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-cli/src/main.rs` | Modified | Add `Bundle`, `Restore` subcommands |
| `crates/anvil-core/src/bundle/` | New | Core bundle/restore logic |
| `crates/anvil-core/Cargo.toml` | Modified | Add `tar`, `flate2`, `sha2` |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Archive format choice | Low | tar+gzip: battle-tested, cross-platform |
| Large workspace context | Low | Only metadata, not files or runtimes |
| Checksum mismatch | Low | Clear error with expected/actual hash |

## Rollback Plan

`anvil bundle` writes only the output file — delete it. `anvil restore` writes manifest+lockfile — `git checkout` or manual delete; runtimes recoverable via `anvil up`.

## Dependencies

- `tar` crate (archive), `flate2` (gzip), `sha2` (checksums)

## Success Criteria

- [ ] `anvil bundle` produces a `.forge` archive from any valid workspace
- [ ] `anvil restore project.forge` recreates manifest+lock and delegates to `anvil up`
- [ ] Deterministic: same workspace → identical archive
- [ ] Checksum verification fails with clear error on tampered archive
- [ ] Secrets never included in bundle
