# Testing Infrastructure Specification

## Purpose

Define the automated testing infrastructure for anvil: runnable integration tests, CI/CD pipeline, and baseline coverage for anvil-core modules. Ensures regressions are caught across platforms before merge.

## Requirements

### Requirement: Runnable Integration Tests

Integration test files `jsonrpc_test.rs` and `mcp_test.rs` MUST be executable via `cargo test` without the `--ignored` flag or a separate `cargo build` step.

The system MUST resolve the anvil binary through `CARGO_BIN_EXE_ANVIL` env var set by Cargo when the test target depends on the binary crate. The `#[ignore]` attribute on each integration test MUST be removed.

#### Scenario: Integration tests pass after cargo build

- GIVEN `cargo build` has completed successfully
- WHEN `cargo test --test jsonrpc_test --test mcp_test` is run
- THEN all 11 tests pass without requiring `--ignored`

#### Scenario: Binary not found produces actionable error

- GIVEN `CARGO_BIN_EXE_ANVIL` is unset
- AND the anvil binary is not in PATH
- WHEN `cargo test --test jsonrpc_test` is run
- THEN the test SHOULD produce a clear error message indicating the binary was not found

#### Scenario: Platform-specific test guard

- GIVEN a test uses OS-specific behavior (e.g., process signaling)
- THEN it SHOULD use `#[cfg(not(target_os = "windows"))]` instead of blanket `#[ignore]`

### Requirement: CI/CD Pipeline

The repository MUST include a GitHub Actions workflow at `.github/workflows/ci.yml` that builds and tests the project on every push to master and every pull request targeting master.

The workflow MUST use a matrix strategy covering `ubuntu-latest`, `macos-latest`, and `windows-latest`. Steps MUST include: checkout, Rust toolchain installation, `cargo build`, `cargo test` (unit + integration), and `cargo clippy`. The workflow SHOULD cache `~/.cargo` and `target/` directories between runs.

#### Scenario: CI triggers on push to master

- GIVEN a push to the master branch
- WHEN the CI workflow runs
- THEN it MUST execute on all 3 OS platforms

#### Scenario: CI triggers on pull request

- GIVEN a pull request targeting master
- WHEN the CI workflow runs
- THEN all matrix jobs MUST succeed before merge

#### Scenario: Cargo dependency caching

- GIVEN the CI workflow runs for the first time
- WHEN `cargo build` completes successfully
- THEN `~/.cargo/registry` and `target/` SHOULD be cached
- AND subsequent runs MUST restore the cache before `cargo build`

#### Scenario: Clippy warnings fail the build

- GIVEN `cargo clippy` runs in CI
- WHEN any clippy warning is emitted
- THEN the step MUST fail (set `RUSTFLAGS` or `-D warnings` equivalent)

### Requirement: Coverage Gap Identification

The system MUST identify anvil-core source files that lack `#[cfg(test)]` test modules. Each untested module MUST be documented with a baseline status: either a smoke test is added, or a `// TODO: tests` marker is left with rationale.

Target modules without tests: `launcher.rs`, `lib.rs`, `lock.rs`, `manifest.rs`, `state.rs`, `api/mod.rs`, `operations/mod.rs`, `plugin/mod.rs`.

#### Scenario: Untested modules are cataloged

- GIVEN a coverage audit of `anvil-core/src/`
- WHEN scanning for `#[cfg(test)]` in each `.rs` file
- THEN modules without test modules MUST be listed in a coverage report

#### Scenario: Smoke test added for high-risk module

- GIVEN an untested module with non-trivial logic (e.g., `installer.rs`, `resolver.rs`)
- WHEN the module already has tests (it does)
- THEN no action needed beyond verifying the existing tests are adequate

#### Scenario: TODO marker left with rationale

- GIVEN a module where adding tests requires significant refactoring (e.g., `lib.rs`)
- WHEN a smoke test is not feasible
- THEN a `// TODO: tests — {reason}` comment MUST be placed at the module root
- AND the coverage report MUST reflect it as a known gap
