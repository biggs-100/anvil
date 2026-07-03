# Proposal: anvil-shims

## Intent
Implement a high-performance multicall shim executable (`anvil-shim`) to route commands (node, python, bun, go, cargo/rust) dynamically based on active workspace configurations, without modifying permanent shell files automatically, using a flat local cache for sub-5ms latency.

## Scope

### In Scope
- Create lightweight multicall binary crate `crates/anvil-shim` (no async, minimal dependencies).
- Install shims via `anvil setup` to `~/.anvil/bin/` with PATH instructions. Add `anvil doctor` check.
- Write flat metadata cache file `.anvil/shims.cache` in TOML containing binary mappings, version signature, and validation timestamps.
- Add cache/state entries incrementally to project `.gitignore` during `anvil init`.
- Forwarding logic: Upward cache traversal -> fallback to system global path -> fallback to user instruction suggestion.
- Process replacement on Unix (`execvp`) and process forwarding on Windows.
- CLI command `anvil which <runtime>` returning detailed path and context source.

### Out of Scope
- Background daemon service (deferred).
- Shell auto-activation cd hooks (direnv-like).

## Capabilities

### New Capabilities
- `multicall-shim`: Pure Rust lightweight execution redirector.
- `shims-installer`: CLI commands to register shims in `~/.anvil/bin` and check PATH via `doctor`.
- `shims-cache-manager`: Serializes/deserializes and updates `.anvil/shims.cache`.
- `observability-which`: `anvil which <runtime>` command.

### Modified Capabilities
- `runtime-manager`: Regenerates the shims cache on successful install/lock updates.

## Approach
- Lightweight Rust binary utilizing `std::process::Command` and `std::os::unix::process::CommandExt` to replace process via `execvp` on Unix, or forward stdin/stdout/stderr/signals and exit code on Windows.
- Traverses parent directories upwards to locate `.anvil/shims.cache` to minimize overhead (avoid lockfile/TOML parsing at runtime).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-shim` | New | Multicall redirector executable. |
| `crates/anvil-cli` | Modified | Add `anvil setup`, `anvil doctor`, and `anvil which`. |
| `crates/anvil-core` | Modified | Cache writing in lock/install updates, gitignore addition in `anvil init`. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PATH Recursion Loop | Low | Filter out `~/.anvil/bin` from environment PATH before forwarding to system tools. |
| Executable Locking | Med | On Windows, copy shim executable instead of using symlinks; exit immediately on forward. |

## Rollback Plan
- Run `anvil setup --uninstall` to remove all binary files from `~/.anvil/bin/` and restore original environment behavior. Manual deletion of `~/.anvil/bin` is also fully effective.

## Dependencies
- Standard Rust library.
- TOML parser for shim cache deserialization in the main CLI.

## Success Criteria
- [ ] Direct tool invocation (e.g. `node`) overhead is <5ms compared to native system execution.
- [ ] Correctly resolves and executes workspace-specific tools inside a project, and falls back cleanly outside it.
- [ ] `anvil doctor` accurately alerts user if `~/.anvil/bin` is not in the system `PATH`.
