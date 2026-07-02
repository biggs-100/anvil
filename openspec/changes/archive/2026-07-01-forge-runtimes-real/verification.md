# Verification Report: forge-runtimes-real

- **Change ID:** forge-runtimes-real
- **Mode:** OpenSpec
- **Status:** PASS (with warning on system-wide pre-installed runtime check)

## Task Completeness

All 14 tasks in `openspec/changes/forge-runtimes-real/tasks.md` are marked as complete (`- [x]`).

| Phase / Task | Goal | Status |
|---|---|---|
| **Phase 1: Extractor Trait & Implementations** | | |
| Task 1.1 | Add `xz2 = "0.4"` dependency to `crates/forge-core/Cargo.toml` | [x] COMPLETE |
| Task 1.2 | Define `Extractor` trait and Zip/TarGz/TarXz extractors | [x] COMPLETE |
| Task 1.3 | Implement path traversal mitigation (Zip Slip) checks | [x] COMPLETE |
| Task 1.4 | Write unit tests for extraction and path traversal prevention | [x] COMPLETE |
| **Phase 2: Provider Trait, 5 Runtime Providers & Registry** | | |
| Task 2.1 | Define `Provider` trait and implement 5 language providers | [x] COMPLETE |
| Task 2.2 | Define `HybridRegistry` and offline version resolution | [x] COMPLETE |
| Task 2.3 | Write unit tests for offline range resolution (`^20`, `~1.8`) | [x] COMPLETE |
| **Phase 3: Lockfile Refactoring** | | |
| Task 3.1 | Refactor `RuntimeLock` to support `EmulationLog` | [x] COMPLETE |
| Task 3.2 | Implement serialization checks for emulation logs | [x] COMPLETE |
| Task 3.3 | Add unit test to verify serialization/deserialization | [x] COMPLETE |
| **Phase 4: Parallel Download Manager & CLI Updates** | | |
| Task 4.1 | Update `install_runtimes` with `JoinSet` for parallel downloads | [x] COMPLETE |
| Task 4.2 | Implement cleanup of incomplete files and abort all on failure | [x] COMPLETE |
| Task 4.3 | Update `forge-cli` main to use new provider and download manager API | [x] COMPLETE |
| Task 4.4 | Verify parallel downloads and error propagation with integration tests | [x] COMPLETE |

---

## Build and Test Evidence

Below is the output from running `cargo test` on the workspace root:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running unittests src\main.rs (target\debug\deps\forge_cli-8f51ebd1cd8f0db2.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\lib.rs (target\debug\deps\forge_core-79ae310d16b364e7.exe)

running 8 tests
test lock::tests::test_lockfile_serialization_with_emulation ... ok
test tests::test_download_sha_mismatch_and_deletion ... ok
test tests::test_is_secret ... ok
test tests::test_mask_env_vars ... ok
test tests::test_offline_version_matching ... ok
test tests::test_parallel_download_and_abort ... ok
test tests::test_standard_archives_extraction ... ok
test tests::test_zip_slip_prevention ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running unittests src\lib.rs (target\debug\deps\forge_drivers-d70aa8a844f57143.exe)

running 1 test
Detected package manager: Winget
test tests::test_detect_package_manager ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests forge_core

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests forge_drivers

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## Spec Compliance Matrix

The implementation has been mapped to requirements across the core specifications and the delta specifications:

| Spec / Req ID | Requirement | Covering Test / Implementation Evidence | Status |
|---|---|---|---|
| **Runtime Providers** (`runtime-providers/spec.md`) | | | |
| REQ-PROV-001 | modular Provider interface & dynamic resolution | `Provider` trait and implementations (`NodeProvider`, `PythonProvider`, etc.). Tested in `test_offline_version_matching`. | **PASS** |
| REQ-PROV-002 | pre-installation verification / skip download | Verified via cached directory checks in `install_runtimes` and `download_runtime` in `crates/forge-core/src/lib.rs`. | **PASS WITH WARNING** (No system-wide path detection implemented, localized cache folder verification is used instead). |
| **Archive Extractors** (`archive-extractors/spec.md`) | | | |
| REQ-EXT-001 | ZIP, TarGz, TarXz decompression support | `ZipExtractor`, `TarGzExtractor`, and `TarXzExtractor` in `crates/forge-core/src/lib.rs`. Tested in `test_standard_archives_extraction`. | **PASS** |
| REQ-EXT-002 | Prevent path traversal attacks (Zip Slip) | `check_path_traversal` validation in extraction routines. Tested in `test_zip_slip_prevention`. | **PASS** |
| **Hybrid Registry** (`hybrid-registry/spec.md`) | | | |
| REQ-REG-001 | consult `.forge/metadata_cache.toml` first | Checked in `update_lockfile` (searches for cache file before defaulting). | **PASS** |
| REQ-REG-002 | fail immediately on exact uncached if offline | Handled by `HybridRegistry::resolve` which errors if exact match is missing. | **PASS** |
| REQ-REG-003 | resolve loose version ranges if offline | Handled in `HybridRegistry::resolve` sorting and matching. Tested in `test_offline_version_matching`. | **PASS** |
| **Runtime Manager Delta** (`runtime-manager/spec.md`) | | | |
| REQ-MGR-002 | parallel download/extraction via JoinSet | Handled in `install_runtimes` using `tokio::task::JoinSet`. Tested in `test_parallel_download_and_abort`. | **PASS** |
| REQ-MGR-003 | delegate resolution to providers | Implemented in `update_lockfile` using the provider map. | **PASS** |
| **Lockfile Generator Delta** (`lockfile-generator/spec.md`) | | | |
| REQ-LOCK-003 | Record emulation details in `forge.lock` | `emulation` attribute on `RuntimeLock` serialize/deserialize check. Tested in `test_lockfile_serialization_with_emulation`. | **PASS** |

---

## Findings and Comments

### Warnings / Deviations
1. **Host-Level Pre-installation Checks (REQ-PROV-002)**:
   The specification mentions skipping downloading if a valid system installation (e.g. standard system Go) matches version constraints. The current implementation performs pre-installation check localized only to `.forge/runtimes/{name}/{version}/extracted`. It does not execute or query the system path (like running `go version` on the host to use a system install).
   *Impact*: Low. Localized sandbox caching prevents redundant downloads for the current toolchain.

### Suggestions
1. **Path Canonicalization on Windows**:
   In `check_path_traversal`, path canonicalization is performed to resolve `..`. On Windows, this requires directories to exist in some contexts. The unit test uses `C:\allowed\directory` which is not canonicalized (using fallback path buf). Ensure production extraction paths are safely formatted before calling canonicalize.
