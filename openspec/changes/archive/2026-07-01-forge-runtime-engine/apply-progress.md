# Apply Progress: Modularize Forge Core Runtime Engine

- **Change**: `forge-runtime-engine`
- **Workload mode**: `size:exception`
- **Status**: All 10 tasks completed successfully, fully compiling and passing tests.

## Completed Tasks

- [x] **1.1 Primitive types**: Created `crates/forge-core/src/types.rs` containing `RuntimeId`, `RuntimeVersion`, `Hash`, `Platform`, `Architecture`, `RuntimeLock`, `EmulationLog`, and `Lockfile`.
- [x] **1.2 Config parsing**: Created `crates/forge-core/src/manifest.rs` containing `ForgeConfig`, `find_forge_toml`, and `load_config`.
- [x] **1.3 Facade update**: Updated `crates/forge-core/src/lib.rs` to expose submodules and pub-use stable interfaces. Move types tests to `types.rs`.
- [x] **2.1 Registry**: Created `crates/forge-core/src/registry.rs` and moved `HybridRegistry`, `RegistryEntry`, and normalization/detection helpers. Moved registry tests.
- [x] **2.2 Resolver**: Created `crates/forge-core/src/resolver.rs` defining the `RuntimeProvider` trait, `Resolver` registry struct, and individual runtime providers (`Node`, `Python`, `Bun`, `Go`, `Rust`).
- [x] **2.3 Cache**: Created `crates/forge-core/src/cache.rs` housing directory configuration, shims map generation, `.gitignore` syncing, and serialization. Moved cache tests.
- [x] **3.1 Installer**: Created `crates/forge-core/src/installer.rs` containing the `Extractor` trait, decompressors (`ZipExtractor`, `TarGzExtractor`, `TarXzExtractor`), `download_runtime`, and `install_runtimes`.
- [x] **3.2 Environment**: Created `crates/forge-core/src/environment.rs` containing PATH lookup, env parser, and secrets masking. Moved env tests.
- [x] **3.3 Launcher**: Created `crates/forge-core/src/launcher.rs` containing `run_command_in_env` and `spawn_shell_in_env`.
- [x] **3.4 Integration**: Extracted consolidated cross-module tests to `crates/forge-core/tests/integration.rs` testing standard archives, Zip Slip prevention, and parallel mock downloads/abortion.

## Created/Modified Files

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/lib.rs` | Modified | Rewritten as a clean facade module for submodules with pub-use re-exports. |
| `crates/forge-core/src/lock.rs` | Modified | Updated to import domain types from `types.rs` instead of defining them inline. |
| `crates/forge-core/src/types.rs` | Created | Domain structs, enums, and lockfile/emulation types. |
| `crates/forge-core/src/manifest.rs` | Created | Configuration loading and parsing helpers. |
| `crates/forge-core/src/registry.rs` | Created | Registry caching, lookup, and platform/architecture detection. |
| `crates/forge-core/src/resolver.rs` | Created | Refactored providers mapping via extensible provider registration registry. |
| `crates/forge-core/src/installer.rs` | Created | Downloader, path traversal validation, and extractors. |
| `crates/forge-core/src/cache.rs` | Created | Cache directory and shims management. |
| `crates/forge-core/src/environment.rs` | Created | Secret masking and env parser. |
| `crates/forge-core/src/launcher.rs` | Created | Process spawning and shell wrappers. |
| `crates/forge-core/tests/integration.rs` | Created | Consolidated integration tests target. |
| `openspec/changes/forge-runtime-engine/tasks.md` | Modified | Checkboxes checked off. |

## Deviations or Issues

None. Clean compiler output without warnings and all 16 workspace tests are fully passing (including unit and integration suites).
