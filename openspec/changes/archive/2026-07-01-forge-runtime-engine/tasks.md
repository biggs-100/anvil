Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Modularize Forge Core Runtime Engine

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 800-1000 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Domain & Manifest Setup | PR 1 | Base branch; relocates config, types, & tests |
| 2 | Resolution & Cache | PR 2 | Extensible resolver, registry, cache logic |
| 3 | Spawning & Integration | PR 3 | Spawn/env contexts, extraction, full tests |

## Phase 1: Domain & Manifest Setup (PR 1)

- [x] 1.1 Create `crates/forge-core/src/types.rs` containing Primitive types (`RuntimeId`, `RuntimeVersion`, `Hash`, `Platform`, `Architecture`, `RuntimeLock`, `EmulationLog`) from `lib.rs`.
- [x] 1.2 Create `crates/forge-core/src/manifest.rs` and move `ForgeConfig`, `find_forge_toml`, `load_config` from `lib.rs`.
- [x] 1.3 Update `crates/forge-core/src/lib.rs` to expose `types` and `manifest`, and re-export stable types. Move unit tests to their respective modules. Verify compiling and passing tests.

## Phase 2: Resolution & Cache Infrastructure (PR 2)

- [x] 2.1 Create `crates/forge-core/src/registry.rs` and relocate registry types (`HybridRegistry`, `RegistryEntry`), normalizations, and `test_offline_version_matching`.
- [x] 2.2 Create `crates/forge-core/src/resolver.rs` defining `RuntimeProvider` trait and refactored provider structs mapping node, python, bun, go, rust in a map.
- [x] 2.3 Create `crates/forge-core/src/cache.rs` to house cache managers (shims map generation, signature/write helper, `.gitignore` sync). Move cache-related unit tests.

## Phase 3: Spawning, Execution, & Integration (PR 3)

- [x] 3.1 Create `crates/forge-core/src/installer.rs` containing `Extractor` trait, `check_path_traversal`, decompressors, `download_runtime`, `install_runtimes`, and archive extraction.
- [x] 3.2 Create `crates/forge-core/src/environment.rs` (PATH manipulation, `.env` file parsing, secret masking).
- [x] 3.3 Create `crates/forge-core/src/launcher.rs` (process spawning and signal forwarding).
- [x] 3.4 Relocate remaining unit tests. Create consolidated integration tests in `crates/forge-core/tests/integration.rs` testing standard archives, zip slip, parallel downloads. Verify all tests pass.
