# Apply Progress: Real Runtime Downloads & Offline-First Registry

**Change**: `forge-runtimes-real`
**Workload Mode**: `size:exception` (Single large PR)

## Completed Tasks

### Phase 1: Extractor Trait & Implementations
- [x] **Task 1.1**: Added dependency `xz2 = "0.1.7"` and `semver = "1.0"` to `crates/anvil-core/Cargo.toml`. *(Note: adjusted version of `xz2` to `0.1.7` because `0.4` does not exist on crates.io).*
- [x] **Task 1.2**: Defined `Extractor` trait and implemented `ZipExtractor`, `TarGzExtractor`, and `TarXzExtractor` (using `xz2`) in `crates/anvil-core/src/lib.rs`.
- [x] **Task 1.3**: Implemented Zip Slip path-traversal prevention checks in all extractors, returning `Err` if paths resolve outside the destination directory.
- [x] **Task 1.4**: Wrote unit tests in `crates/anvil-core/src/lib.rs` testing standard archives (ZIP, TarGz, TarXz) and verifying rejection of path traversal archives.

### Phase 2: Provider Trait, 5 Runtime Providers & Registry
- [x] **Task 2.1**: Defined `Provider` trait and implemented `NodeProvider`, `PythonProvider`, `BunProvider`, `GoProvider`, and `RustProvider` in `crates/anvil-core/src/lib.rs`.
- [x] **Task 2.2**: Defined `HybridRegistry` and implemented offline version resolution (using `VersionReq` matching against `.anvil/metadata_cache.toml` or default internal database).
- [x] **Task 2.3**: Wrote unit tests in `crates/anvil-core/src/lib.rs` to verify offline semver range matching for `^20` and `~1.8` against mock registry configurations.

### Phase 3: Lockfile Refactoring
- [x] **Task 3.1**: Created `crates/anvil-core/src/lock.rs` and refactored `RuntimeLock` to contain an optional `EmulationLog` (with fields: `requested`, `installed`, `reason`).
- [x] **Task 3.2**: Implemented emulation log serialization checks in `crates/anvil-core/src/lock.rs` using `#[serde(skip_serializing_if = "Option::is_none")]`.
- [x] **Task 3.3**: Added unit test to verify that `anvil.lock` serializes/deserializes emulation logs correctly.

### Phase 4: Parallel Download Manager & CLI Updates
- [x] **Task 4.1**: Updated `install_runtimes` in `crates/anvil-core/src/lib.rs` using `tokio::task::JoinSet` to run download, checksum validation, and extraction tasks in parallel.
- [x] **Task 4.2**: Added cleanup guards (`FileCleanupGuard`, `DirCleanupGuard`) to delete partial/incomplete files and folders on failure, and implemented cancellation (`JoinSet::abort_all()`).
- [x] **Task 4.3**: Updated `crates/anvil-cli/src/main.rs` to use new provider and download manager API, supporting and logging emulation fallbacks.
- [x] **Task 4.4**: Verified execution of parallel downloads and error propagation using integration tests with a mock web server.

---

## Created/Modified Files

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/Cargo.toml` | Modified | Added `xz2` and `semver` dependencies. |
| `crates/anvil-core/src/lock.rs` | Created | Defined `RuntimeLock`, `Lockfile`, `EmulationLog`, load/save lockfile, and serialization tests. |
| `crates/anvil-core/src/lib.rs` | Modified | Added `Extractor` and `Provider` traits and implementations, `HybridRegistry`, `FileCleanupGuard`/`DirCleanupGuard`, parallel `JoinSet` installer, and test suites. |
| `crates/anvil-cli/src/main.rs` | Modified | Updated CLI commands (`Lock`, `Run`, `Shell`) to log emulation warnings and run concurrently. |
| `openspec/changes/forge-runtimes-real/tasks.md` | Modified | Marked all 14 tasks complete. |

## Status

All tasks are complete. Compilation succeeds and all 9 test suites pass successfully.
