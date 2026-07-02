# Tasks: forge-bundle

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 365–415 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | single PR |
| Delivery strategy | single-pr-default |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Medium

## Phase 1: Bundle Module

- [x] 1.1 Create `crates/forge-core/src/bundle.rs` — `BundleMetadata`, `ChecksumEntry`, `BundleChecksums`, `BundleError` types
- [x] 1.2 Implement `create_bundle()` — read forge.toml + forge.lock, compute SHA-256 per entry
- [x] 1.3 Implement deterministic tar+gzip — sorted entries, mtime=0, no filename header in gzip
- [x] 1.4 Implement `metadata.json` builder — forge_version, created_at, workspace_id, runtime_count, excluded_patterns
- [x] 1.5 Add `pub mod bundle;` and re-exports in `crates/forge-core/src/lib.rs`

## Phase 2: Restore

- [x] 2.1 Implement `restore_bundle()` — decompress, extract to temp dir, read bundle.sha256
- [x] 2.2 Implement `verify_checksums()` — compare SHA-256 of each entry against checksum manifest
- [x] 2.3 Implement atomic rename — move forge.toml + forge.lock from temp dir to workspace
- [x] 2.4 Wire `forge up` delegation after successful extraction

## Phase 3: CLI Wiring

- [x] 3.1 Add `Bundle` / `Restore` variants to `Commands` enum in `crates/forge-cli/src/main.rs`
- [x] 3.2 Add `Bundle` match arm — calls `forge_core::bundle::create_bundle()` with `--output` flag
- [x] 3.3 Add `Restore` match arm — calls `forge_core::bundle::restore_bundle()` with `--force` flag
- [x] 3.4 Update `BUILTIN_COMMANDS` list to include `"bundle"` and `"restore"`

## Phase 4: Testing

- [x] 4.1 Inline `#[cfg(test)]` module in `bundle.rs` — test deterministic archive with mock forge.toml/lock
- [x] 4.2 Test `verify_checksums` — corrupted entry → `ChecksumMismatch`, missing entry → `MissingEntry`, all match → `Ok`
- [x] 4.3 Test gzip determinism — same inputs → byte-identical archives
- [x] 4.4 Test secrets exclusion — `.forge/`, `forge.secrets`, `forge.env` not in archive
