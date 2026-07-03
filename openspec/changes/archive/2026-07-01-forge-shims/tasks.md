Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: anvil-shims

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 350-450 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Create multicall `anvil-shim` & unit tests | PR 1 | Base crate setup, tests/docs |
| 2 | Serializer in `anvil-core` & gitignore setup | PR 2 | Trigger cache rewrite |
| 3 | Install CLI commands `setup`, `doctor`, `which` | PR 3 | Integration tests |

## Phase 1: Crate Setup & multicall shim (PR 1)
- [x] 1.1 Create `crates/anvil-shim/Cargo.toml` with minimal dependencies.
- [x] 1.2 Implement name interception (`current_exe()`) and parent directory traversal searching for `.anvil/shims.cache` in `crates/anvil-shim/src/main.rs`.
- [x] 1.3 Add custom line-by-line key-value parsing of the cache in `crates/anvil-shim/src/main.rs`.
- [x] 1.4 Implement PATH loop recursion prevention in `crates/anvil-shim/src/main.rs` by removing `current_exe()` parent directory from `PATH` before host fallback execution.
- [x] 1.5 Add `execvp` process image replacement on Unix (`CommandExt::exec()`) and stdio/exit code process forwarding on Windows in `crates/anvil-shim/src/main.rs`.
- [x] 1.6 Write unit tests for traversal, key-value parsing, and PATH filtering under `crates/anvil-shim/src/main.rs`. Verify with `cargo test -p anvil-shim`.

## Phase 2: Cache Serialization & gitignore Setup (PR 2)
- [x] 2.1 Register `crates/anvil-shim` in workspace `Cargo.toml`.
- [x] 2.2 In `crates/anvil-core/src/lib.rs`, implement `shims.cache` custom line-by-line key-value serialization.
- [x] 2.3 Integrate cache serialization trigger in `crates/anvil-core/src/lib.rs` upon successful installations or lock updates.
- [x] 2.4 Add helper in `crates/anvil-core/src/lib.rs` to append `.anvil/shims.cache` and `.anvil/state.json` to `.gitignore` during `anvil init`.
- [x] 2.5 Write unit tests verifying cache serialization and gitignore updates in `crates/anvil-core/src/lib.rs`. Verify with `cargo test -p anvil-core`.

## Phase 3: CLI Commands & Verification (PR 3)
- [x] 3.1 Implement command `anvil setup` in `crates/anvil-cli/src/main.rs` to copy `anvil-shim` executable to `~/.anvil/bin` under different runtime aliases (e.g. node, python).
- [x] 3.2 Implement PATH verification logic in `anvil doctor` command under `crates/anvil-cli/src/main.rs` to check if `~/.anvil/bin` is in the environment `PATH`.
- [x] 3.3 Implement `anvil which <runtime>` CLI command under `crates/anvil-cli/src/main.rs` to resolve runtime paths.
- [x] 3.4 Write integration tests under `tests/` or `crates/anvil-cli/` simulating shell forwarding, args propagation, and exit status matching. Verify with `cargo test -p anvil-cli`.

## Remediation (Verification Fixes)
- [x] R.1 Add --uninstall flag and logic to `anvil setup` and write integration tests.
- [x] R.2 Validate version header signature in `read_shims_cache` and write unit/integration tests for invalidation.

