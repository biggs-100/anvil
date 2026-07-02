# Proposal: forge-shims

## Intent
Implement a high-performance multicall shim executable (`forge-shim`) to route commands (node, python, bun, go, cargo/rust) dynamically based on active workspace configurations, without modifying permanent shell files automatically, using a flat local cache for sub-5ms latency.

## Scope

### In Scope
- Create lightweight multicall binary crate `crates/forge-shim` (no async, minimal dependencies).
- Install shims via `forge setup` to `~/.forge/bin/` with PATH instructions. Add `forge doctor` check.
- Write flat metadata cache file `.forge/shims.cache` in TOML containing binary mappings, version signature, and validation timestamps.
- Add cache/state entries incrementally to project `.gitignore` during `forge init`.
- Forwarding logic: Upward cache traversal -> fallback to system global path -> fallback to user instruction suggestion.
- Process replacement on Unix (`execvp`) and process forwarding on Windows.
- CLI command `forge which <runtime>` returning detailed path and context source.

### Out of Scope
- Background daemon service (deferred).
- Shell auto-activation cd hooks (direnv-like).

## Capabilities

### New Capabilities
- `multicall-shim`: Pure Rust lightweight execution redirector.
- `shims-installer`: CLI commands to register shims in `~/.forge/bin` and check PATH via `doctor`.
- `shims-cache-manager`: Serializes/deserializes and updates `.forge/shims.cache`.
- `observability-which`: `forge which <runtime>` command.

### Modified Capabilities
- `runtime-manager`: Regenerates the shims cache on successful install/lock updates.

## Approach
- Lightweight Rust binary utilizing `std::process::Command` and `std::os::unix::process::CommandExt` to replace process via `execvp` on Unix, or forward stdin/stdout/stderr/signals and exit code on Windows.
- Traverses parent directories upwards to locate `.forge/shims.cache` to minimize overhead (avoid lockfile/TOML parsing at runtime).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-shim` | New | Multicall redirector executable. |
| `crates/forge-cli` | Modified | Add `forge setup`, `forge doctor`, and `forge which`. |
| `crates/forge-core` | Modified | Cache writing in lock/install updates, gitignore addition in `forge init`. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PATH Recursion Loop | Low | Filter out `~/.forge/bin` from environment PATH before forwarding to system tools. |
| Executable Locking | Med | On Windows, copy shim executable instead of using symlinks; exit immediately on forward. |

## Rollback Plan
- Run `forge setup --uninstall` to remove all binary files from `~/.forge/bin/` and restore original environment behavior. Manual deletion of `~/.forge/bin` is also fully effective.

## Dependencies
- Standard Rust library.
- TOML parser for shim cache deserialization in the main CLI.

## Success Criteria
- [ ] Direct tool invocation (e.g. `node`) overhead is <5ms compared to native system execution.
- [ ] Correctly resolves and executes workspace-specific tools inside a project, and falls back cleanly outside it.
- [ ] `forge doctor` accurately alerts user if `~/.forge/bin` is not in the system `PATH`.
