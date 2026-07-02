# Tasks: Forge Supply Chain Security

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~450–550 |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: GPG signing; PR 2: Pin-by-hash + audit |
| Delivery strategy | single-pr |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | GPG signature verification | PR 1 | new `gpg.rs`, modify `registry.rs`; standalone |
| 2 | Pin-by-hash + forge audit | PR 2 | modify `manifest.rs`, `installer.rs`, `types.rs`; new `audit.rs`, modify `main.rs` |

## Phase 1: Registry Signing

- [x] 1.1 Create `crates/forge-core/src/gpg.rs` — `EMBEDDED_PUBLIC_KEY` constant, `verify_gpg_signature()` calling `gpg --verify` via `std::process::Command`
- [x] 1.2 Add `FORGE_TRUSTED_KEYS` env var parsing in `gpg.rs` — build temp keyring from embedded key + env entries
- [x] 1.3 Add `mod gpg` to `crates/forge-core/src/lib.rs`
- [x] 1.4 Modify `crates/forge-core/src/registry.rs` — fetch `metadata.toml.asc`, call `verify_gpg_signature()` before cache, fallback to `load_cached()` on failure

## Phase 2: Pin by Hash

- [x] 2.1 Add `RuntimeEntry` enum (`Bare(String)` | `Pinned{version,sha256}`) with `#[serde(untagged)]` in `crates/forge-core/src/manifest.rs`
- [x] 2.2 Update `update_lockfile()` — pass manifest `sha256` pin through to `RuntimeLock.sha256` so installer verifies against it

## Phase 3: Audit Log

- [x] 3.1 Add `download_url`, `file_size`, `sha256`, `verified` optional fields to `Event` in `crates/forge-core/src/types.rs` (`#[serde(default)]`)
- [x] 3.2 Create `crates/forge-cli/src/audit.rs` — read `.forge/journal.jsonl`, filter `Sync`/`Download`/`Extract`/`Commit`, format table with timestamp/runtime/version/operation/url/size/sha256/verified
- [x] 3.3 Add `--json` flag to audit — output JSON array
- [x] 3.4 Wire `Audit` variant in `crates/forge-cli/src/main.rs` — CLI enum, `run_cli()` dispatch, `BUILTIN_COMMANDS`

## Phase 4: Testing

- [x] 4.1 Unit test: GPG `parse_trusted_keys_env`, `extract_key_id` — valid/invalid key formats
- [x] 4.2 Unit test: `RuntimeEntry` deserialization — bare string, pinned object, roundtrip
- [x] 4.3 Unit test: Installer hash mismatch aborts extraction, shows both hashes (existing test maintained)
- [x] 4.4 Unit test: Event optional field JSON roundtrip (`#[serde(default)]` compat)
- [x] 4.5 Integration test: `forge audit` on empty/missing journal returns empty entries
- [x] 4.6 Integration test: Bad GPG sig falls back to cached metadata (via `verify_metadata_signature` logic in registry.rs)
