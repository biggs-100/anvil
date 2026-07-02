Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Real Runtime Downloads & Offline-First Registry

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 400-500 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Extractor trait, Zip/TarGz/TarXz decoders (xz2), and Zip Slip prevention | PR 1 | Base branch; includes unit tests for decompression and zip slip protection |
| 2 | Provider trait, 5 language providers, offline matching in Hybrid Registry | PR 2 | Registry logic and provider resolution; depends on PR 1 |
| 3 | Refactor Lockfile to support EmulationLog (requested, installed, reason) | PR 3 | Refactors crates/forge-core/src/lock.rs and lock representation |
| 4 | Asynchronous parallel download manager via Tokio JoinSet, CLI integration | PR 4 | Integrates all components in crates/forge-core/src/lib.rs and crates/forge-cli/src/main.rs |

## Phase 1: Extractor Trait & Implementations (PR 1)

- [x] 1.1 Add `xz2 = "0.4"` dependency to `crates/forge-core/Cargo.toml`.
- [x] 1.2 Define `Extractor` trait and implement `ZipExtractor`, `TarGzExtractor`, and `TarXzExtractor` in `crates/forge-core/src/lib.rs`.
- [x] 1.3 Implement path traversal mitigation (Zip Slip) checks in extraction logic, returning `Err` if paths escape destination directory.
- [x] 1.4 Write unit tests in `crates/forge-core/src/lib.rs` testing standard archives (ZIP, TarGz, TarXz) and verifying rejection of path traversal archives containing `../`.

## Phase 2: Provider Trait, 5 Runtime Providers & Registry (PR 2)

- [x] 2.1 Define `Provider` trait and implement `NodeProvider`, `PythonProvider`, `BunProvider`, `GoProvider`, and `RustProvider` in `crates/forge-core/src/lib.rs`.
- [x] 2.2 Define `HybridRegistry` and implement offline version resolution (using `VersionReq` matching against `.forge/metadata_cache.toml` or fallback).
- [x] 2.3 Write unit tests in `crates/forge-core/src/lib.rs` to verify offline semver range matching for `^20` and `~1.8` against mock registry configurations.

## Phase 3: Lockfile Refactoring (PR 3)

- [x] 3.1 Create `crates/forge-core/src/lock.rs` (or define in `lib.rs`) and refactor `RuntimeLock` to contain an optional `EmulationLog` (fields: `requested`, `installed`, `reason`).
- [x] 3.2 Implement emulation log serialization checks in `crates/forge-core/src/lock.rs`.
- [x] 3.3 Add unit test to verify that `forge.lock` serializes/deserializes emulation logs correctly.

## Phase 4: Parallel Download Manager & CLI Updates (PR 4)

- [x] 4.1 Update `install_runtimes` in `crates/forge-core/src/lib.rs` using `tokio::task::JoinSet` to run download, checksum validation, and extraction tasks in parallel.
- [x] 4.2 Add error abortion using `JoinSet::abort_all()` and cleanup of incomplete files on task failure.
- [x] 4.3 Update `crates/forge-cli/src/main.rs` to use new provider and download manager API, supporting emulation fallbacks.
- [x] 4.4 Verify execution of parallel downloads and error propagation using integration tests with a mock web server.
