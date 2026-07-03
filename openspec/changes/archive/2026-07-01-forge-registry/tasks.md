# Tasks: Anvil Runtime Registry (ARRS) + Remote Client

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~380-450 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | single PR |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Medium

## Phase 1: ARRS Types

- [x] 1.1 Add `ArrsArtifact` struct to `crates/anvil-core/src/registry.rs` with `platform`, `arch`, `url`, `size`, `sha256` — serde Deserialize + `#[serde(deny_unknown_fields)]`
- [x] 1.2 Add `ArrsMetadata` struct to `registry.rs` with `name`, `version`, `license`, `homepage`, `artifacts: Vec<ArrsArtifact>`, `dependencies: Option<Vec<String>>`
- [x] 1.3 Add `ArrsIndex` struct to `registry.rs` with `toolchains: HashMap<String, ArrsIndexEntry>` — re-export from `lib.rs`
- [x] 1.4 Add serde `deny_unknown_fields` = false (forward compat per REQ-ARRS-004)

## Phase 2: RemoteRegistry

- [x] 2.1 Add `RemoteRegistry` struct to `registry.rs` with `base_url`, `cache_dir`, `client: reqwest::Client`, `ttl: Duration`
- [x] 2.2 Implement `new(base_url, cache_dir)`, `with_ttl()`, `load_cached(name, version)` sync file read from `.anvil/metadata_cache/{name}/{version}/metadata.toml`
- [x] 2.3 Implement `async fetch_metadata(name, version)` — HTTP GET `{base_url}/{name}/{version}/metadata.toml`, parse, cache to disk
- [x] 2.4 Implement `async fetch_index()` — HTTP GET `{base_url}/index.toml`, parse into `ArrsIndex`
- [x] 2.5 Implement TTL check via file mtime — stale entries warn but serve on network failure

## Phase 3: HybridRegistry Chain

- [x] 3.1 Add `remote: Option<RemoteRegistry>` and `cache_dir: Option<PathBuf>` fields to `HybridRegistry`
- [x] 3.2 Add `with_remote()` and `with_cache_dir()` builder methods
- [x] 3.3 Extend `HybridRegistry::resolve()` chain: (1) flat entries, (2) local ARRS cache dir via `load_cached()`, (3) ARM64→x86_64 fallback, (4) embedded `default_with_internal()`
- [x] 3.4 Add `async fn refresh_remote(&self, name, version)` to `HybridRegistry` — calls `RemoteRegistry::fetch_metadata()` on miss

## Phase 4: Update Wiring

- [x] 4.1 Update `update_lockfile()` in `crates/anvil-core/src/lib.rs` to build `HybridRegistry` with `cache_dir` and call `refresh_remote()` per runtime before resolution
- [x] 4.2 Add `ANVIL_REGISTRY_URL` env var support and `[registry] url` config parsing in `HybridRegistry::new()` chain

## Phase 5: CLI

- [x] 5.1 Add `Registry` subcommand variant to `Commands` enum in `crates/anvil-cli/src/main.rs`
- [x] 5.2 Add `anvil registry refresh` that clears cache dir and re-fetches from remote
- [x] 5.3 Wire `Registry` dispatch in `run_cli()` match block

## Phase 6: Testing

- [x] 6.1 Unit test: parse `ArrsMetadata` from TOML + ignore unknown fields
- [x] 6.2 Unit test: parse `ArrsIndex` from TOML
- [x] 6.3 Unit test: cache TTL logic — set mtime, verify stale vs fresh
- [x] 6.4 Unit test: resolve chain order with mocked layers
- [ ] 6.5 Integration test: `RemoteRegistry::fetch_metadata()` with mock HTTP server (reuse `start_mock_server` pattern)
- [ ] 6.6 Integration test: resolve from cache when remote unreachable
