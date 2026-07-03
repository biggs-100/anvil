# Design: Anvil Testing Infrastructure

## Technical Approach

Three-phase rollout: (1) un-ignore integration tests by fixing the `CARGO_BIN_EXE_` env var resolution and adding a platform guard for the kill-based lifecycle test, (2) add a GitHub Actions matrix CI workflow with caching and clippy enforcement, (3) document anvil-core coverage gaps with TODO markers — no automatic test generation.

The key insight: Cargo already sets `CARGO_BIN_EXE_ANVIL_CLI` (uppercase, underscore) for integration tests in a package that produces a binary. The tests' current fallback chain checks for wrong var names and never matches, forcing `#[ignore]`. Fix the var name and the tests run automatically.

## Architecture Decisions

| Decision | Choice | Alternatives | Rationale |
|----------|--------|-------------|-----------|
| Binary resolution env var | `CARGO_BIN_EXE_ANVIL_CLI` | Current lowercase-hyphen `CARGO_BIN_EXE_anvil-cli` | Cargo uppercases and underscores binary names — current code never matches, so `#[ignore]` was the only way to avoid failures |
| OS guard for kill test | `#[cfg(not(target_os = "windows"))]` on `test_subprocess_lifecycle_error` | Guard all tests, keep `#[ignore]` | `child.kill()` + broken-pipe write is Windows-specific; other tests use stdin/stdout which work identically cross-platform |
| CI caching | `Swatinem/rust-cache` | Manual `~/.cargo` + `target/` restore | Standard action handles cache key invalidation, workspace detection, and incremental builds automatically |
| Clippy enforcement | `-D warnings` in `RUSTFLAGS` | Separate `cargo clippy` step with `--deny warnings` | Both work; env var approach is simpler and catches compiler warnings too |
| Coverage gap treatment | TODO markers only | Add smoke tests for every module | Modules like `launcher.rs`, `lock.rs` need significant refactoring to be testable — forcing tests now would bloat scope and risk churn |

## Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      CI Pipeline                             │
│                                                              │
│  git push/PR ──→ checkout ──→ toolchain ──→ cache restore    │
│                       │                                      │
│                   cargo build ────→ cargo test ────→ clippy  │
│                       │              │    │                  │
│                  target/debug/    unit   integration         │
│                  anvil-cli.exe    tests  tests (11)          │
│                                     │                        │
└─────────────────────────────────────┼────────────────────────┘
                                      │
                    CARGO_BIN_EXE_ANVIL_CLI ──→ spawn anvil-cli
                                                  │
                                              stdin/stdout
                                                  │
                                              JSON-RPC 2.0
```

Integration test flow: `cargo test` → Cargo sets `CARGO_BIN_EXE_ANVIL_CLI` env var → `forge_exe()` resolves binary path → `Command::new()` spawns `anvil jsonrpc` or `anvil mcp` → test writes JSON-RPC request to stdin → reads response from stdout → asserts expected fields.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `.github/workflows/ci.yml` | Create | Matrix CI: ubuntu/macos/windows, build → test → clippy, cache, fail on warnings |
| `crates/anvil-cli/tests/jsonrpc_test.rs` | Modify | Fix `CARGO_BIN_EXE_anvil-cli` → `CARGO_BIN_EXE_ANVIL_CLI`; remove `#[ignore]` from all 5 tests; add `#[cfg(not(target_os = "windows"))]` to `test_subprocess_lifecycle_error` |
| `crates/anvil-cli/tests/mcp_test.rs` | Modify | Fix `CARGO_BIN_EXE_anvil-cli` → `CARGO_BIN_EXE_ANVIL_CLI`; remove `#[ignore]` from all 6 tests |
| `crates/anvil-core/src/launcher.rs` | Modify | Add `// TODO: tests — requires mocking OS process APIs` at module root |
| `crates/anvil-core/src/lock.rs` | Modify | Add `// TODO: tests — requires filesystem isolation fixture` at module root |
| `crates/anvil-core/src/manifest.rs` | Modify | Add `// TODO: tests — requires TOML parsing fixtures` at module root |
| `crates/anvil-core/src/state.rs` | Modify | Add `// TODO: tests — depends on Engine lifecycle` at module root |
| `crates/anvil-core/src/lib.rs` | Modify | Add `// TODO: tests — re-export facade; test coverage in submodules` |
| `crates/anvil-core/src/operations/mod.rs` | Modify | Add `// TODO: tests — operations module` |
| `crates/anvil-core/src/api/mod.rs` | Modify | Add `// TODO: tests — API surface is tested via v1.rs integration` |
| `crates/anvil-core/src/plugin/mod.rs` | Modify | Add `// TODO: tests — registry.rs has unit tests; plugin lifecycle needs e2e` |

## Interfaces / Contracts

No new interfaces. The `forge_exe()` helper changes its env var lookup chain:

```rust
fn forge_exe() -> String {
    // Cargo sets CARGO_BIN_EXE_ANVIL_CLI (uppercase, underscore-separated)
    // when integration tests depend on the same package that produces the binary.
    std::env::var("CARGO_BIN_EXE_ANVIL_CLI")
        .unwrap_or_else(|_| "anvil-cli".to_string())
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | forge-exe resolution | Not needed — env var set by Cargo, tested via integration suite |
| Integration | JSON-RPC and MCP protocols | Un-ignore 11 existing tests, run via `cargo test --test jsonrpc_test --test mcp_test` |
| CI pipeline | Workflow correctness | Verify on push to non-master branch before merging; check all 3 OS matrix jobs pass |
| Platform | Windows-specific kill test | Guarded with `#[cfg(not(target_os = "windows"))]` — not run on Windows |

## Migration / Rollout

No data migration. Rollout order:
1. Fix env var names in both test files, remove `#[ignore]`, add platform guard — verify locally with `cargo build && cargo test --test jsonrpc_test --test mcp_test`
2. Add CI workflow in a feature branch, push to verify the matrix runs
3. Add TODO markers to anvil-core modules
4. Open a single PR containing all changes

Rollback: revert CI workflow file, re-add `#[ignore]` to tests.

## Open Questions

- [ ] Verify `CARGO_BIN_EXE_ANVIL_CLI` casing locally — Cargo docs say uppercase+underscore, but empirical check on the actual toolchain is needed
- [ ] Confirm `child.kill()` behavior on macOS for `test_subprocess_lifecycle_error` — Linux is covered, macOS needs manual check if CI isn't available yet
