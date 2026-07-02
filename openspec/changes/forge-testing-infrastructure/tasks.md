# Tasks: Forge Testing Infrastructure

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~140–160 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | single PR |
| Delivery strategy | single-pr |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Integration Tests — Fix Binary Resolution

- [x] 1.1 `crates/forge-cli/tests/jsonrpc_test.rs` — Change `forge_exe()` to use `CARGO_BIN_EXE_FORGE_CLI` (uppercase underscore); remove `CARGO_BIN_EXE_forge-cli` fallback
- [x] 1.2 `crates/forge-cli/tests/jsonrpc_test.rs` — Remove `#[ignore]` from all 5 tests; add `#[cfg(not(target_os = "windows"))]` to `test_subprocess_lifecycle_error`
- [x] 1.3 `crates/forge-cli/tests/mcp_test.rs` — Change `forge_exe()` to use `CARGO_BIN_EXE_FORGE_CLI`; remove fallback chain
- [x] 1.4 `crates/forge-cli/tests/mcp_test.rs` — Remove `#[ignore]` from all 6 tests
- [x] 1.5 Update top doc comments in both test files to remove `#[ignore]` references

## Phase 2: CI/CD — GitHub Actions Pipeline

- [x] 2.1 Create `.github/workflows/ci.yml` with `on: [push, pull_request]` targeting master
- [x] 2.2 Add matrix strategy: `os: [ubuntu-latest, macos-latest, windows-latest]`, `rust: [stable]`
- [x] 2.3 Add step: `Swatinem/rust-cache@v2` for `~/.cargo` + `target/` caching
- [x] 2.4 Add step: `cargo build` with `RUSTFLAGS: "-D warnings"`
- [x] 2.5 Add step: `cargo test` (unit + integration tests)
- [x] 2.6 Add step: `cargo clippy` with `-- -D warnings`

## Phase 3: Coverage Gaps — TODO Markers

- [x] 3.1 Add `// TODO: tests` comment to `crates/forge-core/src/launcher.rs` — requires mocking OS process APIs
- [x] 3.2 Add `// TODO: tests` comment to `crates/forge-core/src/lock.rs` — requires filesystem isolation fixture
- [x] 3.3 Add `// TODO: tests` comment to `crates/forge-core/src/manifest.rs` — requires TOML parsing fixtures
- [x] 3.4 Add `// TODO: tests` comment to `crates/forge-core/src/state.rs` — depends on Engine lifecycle
- [x] 3.5 Add `// TODO: tests` comment to `crates/forge-core/src/lib.rs` — re-export facade
- [x] 3.6 Add `// TODO: tests` comment to `crates/forge-core/src/operations/mod.rs` — operations module
- [x] 3.7 Add `// TODO: tests` comment to `crates/forge-core/src/api/mod.rs` — API surface tested via v1.rs integration
- [x] 3.8 Add `// TODO: tests` comment to `crates/forge-core/src/plugin/mod.rs` — registry.rs has unit tests, lifecycle needs e2e

## Phase 4: Verification

- [ ] 4.1 Verify locally: `cargo build && cargo test --test jsonrpc_test --test mcp_test` passes without `--ignored`
- [ ] 4.2 Verify CI workflow syntax: `act` dry-run or push to feature branch
- [ ] 4.3 Confirm all forge-core TODO markers compile (comments only, no code impact)
