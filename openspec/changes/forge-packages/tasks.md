# Tasks: Forge Packages

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~130–170 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr-default |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: Config / Foundation

- [x] 1.1 Add `PackagesConfig` struct with `pip: Option<String>` to `crates/forge-core/src/manifest.rs`
- [x] 1.2 Add `pub packages: Option<PackagesConfig>` field to `ForgeConfig` with `#[serde(default)]`
- [x] 1.3 Add `pub mod packages;` and `pub use packages::install_pip_deps;` to `crates/forge-core/src/lib.rs`

## Phase 2: Installer Module

- [x] 2.1 Create `crates/forge-core/src/packages.rs` with `install_pip_deps(workspace_root, cache_dir) -> Result<(), String>`
- [x] 2.2 Resolve forge-managed python binary: search lockfile for `python` runtime, build `extracted/bin/python[3]` path from cache_dir
- [x] 2.3 Validate requirements.txt exists from `packages.pip` path relative to workspace_root; return clear error if missing
- [x] 2.4 Spawn `python[3] -m pip install -r <requirements.txt>` via `run_command_in_env` with `bin_dirs` pointing to python's extracted bin dir

## Phase 3: Wiring in forge-cli

- [x] 3.1 Import `forge_core::install_pip_deps` in `crates/forge-cli/src/main.rs`
- [x] 3.2 After `engine.sync().await?;` in `Commands::Sync` handler, call `install_pip_deps(&workspace_root, &cache_dir)?`
- [x] 3.3 After `engine.sync().await?;` in `Commands::Up` handler, call `install_pip_deps(&workspace_root, &cache_dir)?`

## Phase 4: Testing

- [x] 4.1 Test: `PackagesConfig` TOML deserialization — parse `[packages]\npip = "reqs.txt"` and assert `pip == Some("reqs.txt")`
- [x] 4.2 Test: `install_pip_deps` returns `Ok(())` when no packages config present
- [x] 4.3 Test: `install_pip_deps` returns error when pip configured but no python in lockfile
- [x] 4.4 Test: `install_pip_deps` returns error when requirements.txt file does not exist
