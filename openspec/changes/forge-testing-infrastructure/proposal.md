# Proposal: Forge Testing Infrastructure

## Intent

186 unit tests exist but 11 integration tests are silently skipped (`#[ignore]`), there's no CI/CD pipeline, and coverage gaps are invisible. This leaves regressions undetected across platforms and SDK boundaries.

## Scope

### In Scope
- Make `jsonrpc_test.rs` (5) and `mcp_test.rs` (6) runnable via Cargo's `CARGO_BIN_EXE_` — remove `#[ignore]`
- Add GitHub Actions workflow: build + test on Windows, macOS, Linux
- Audit forge-core sources for untested modules; add baseline coverage

### Out of Scope
- Cross-SDK parity tests (Go/Python/TS) — manual only, deferred
- Performance benchmarks or load tests
- Coverage threshold enforcement in CI (visibility only)

## Capabilities

### New Capabilities
- `testing-infrastructure`: CI/CD pipeline definition and integration test framework for forge-cli

### Modified Capabilities
- None — this is a greenfield spec for automated testing infrastructure

## Approach

**Phase 1**: Refactor integration tests — already uses `CARGO_BIN_EXE_forge` env var. Remove `#[ignore]`, add `#[cfg(not(target_os = "windows"))]` guards where needed, verify `cargo test --test jsonrpc_test` passes post-build.

**Phase 2**: Create `.github/workflows/ci.yml` — matrix strategy (os: [windows, macos, ubuntu], rust: [stable]). Steps: checkout, toolchain, build, unit tests, integration tests (depends on build artifacts). Fail on warnings via `RUSTFLAGS`.

**Phase 3**: Audit `forge-core/src/` — files without `#[cfg(test)]` (launcher.rs, lock.rs, manifest.rs, operations/mod.rs). Add smoke tests or TODO markers. Report coverage delta.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-cli/tests/jsonrpc_test.rs` | Modified | Remove `#[ignore]`, ensure `CARGO_BIN_EXE_` always resolves |
| `crates/forge-cli/tests/mcp_test.rs` | Modified | Same as above |
| `.github/workflows/ci.yml` | New | Matrix build+test on 3 OSes |
| `forge-core/src/*.rs` | Modified | Add test modules for untested files |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| MacOS/Linux-specific test failures | Medium | Add `cfg(not(target_os))` guards, document platform quirks |
| CI minutes cost on multi-OS matrix | Low | Use `rust-os-check` action; skip redundant builds |

## Rollback Plan

- Revert `.github/workflows/ci.yml` to disable CI
- Add `#[ignore]` back to integration tests if they flake on non-Windows

## Dependencies

- GitHub repository with Actions enabled
- `cargo test` passes locally on at least one platform

## Success Criteria

- [ ] `cargo test --test jsonrpc_test` passes after `cargo build` (no `--ignored` flag)
- [ ] `cargo test --test mcp_test` passes after `cargo build`
- [ ] CI workflow triggers on push/PR to main, runs all 3 OSes
- [ ] forge-core source files without tests identified and documented
