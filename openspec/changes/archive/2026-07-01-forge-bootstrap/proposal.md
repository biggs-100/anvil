# Proposal: Forge Bootstrap

## Intent
Bootstrap the core architecture and first implementation slice of Forge as a Runtime Environment Manager (not a package manager) that is fast, deterministic, cross-platform, and optimized for humans and AI agents.

## Scope

### In Scope
- Forge CLI and Runtime Engine split from the Platform/Driver layers.
- Native Rust implementation.
- Support for downloading and executing 5 native runtimes: Python, Node.js, Bun, Go, and Rust.
- System packages fallback wrapper drivers executing host package managers (Winget, Homebrew, Apt/Pacman).
- Core activation via subprocess wrapping (`forge run <cmd>` and `forge shell`). No shell hooks.
- Env loading from `forge.env` and secret verification (checked, not displayed) in `forge doctor` / `forge ai context`.
- Deterministic `forge.lock` (versions, platforms, URLs, sizes, SHA-256 hashes).

### Out of Scope
- Heavy virtualization (Docker wrapper, WSL integration).
- Shell auto-switching cd hooks (e.g. direnv style).
- Secure OS Keychain integration.
- Forge custom package registry.

## Capabilities

### New Capabilities
- `runtime-manager`: Resolves, downloads, extracts, and executes local toolchains.
- `platform-drivers`: Fallback wrapper for system package manager installations.
- `environment-activation`: Subprocess environment injector and `forge.env` / secrets parser.
- `lockfile-generator`: Generates, parses, and synchronizes `forge.lock`.
- `agent-inspector`: Provides `forge ai context` and `forge ai doctor` commands with structured JSON outputs.

### Modified Capabilities
- None

## Approach
Implement a native Rust workspace split into core CLI/Engine and driver crates. The engine resolves runtime binaries, writes to a directory-local `.forge/` cache, updates `forge.lock`, and spawns command execution wrappers.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `Cargo.toml` | New | Root Rust workspace definition. |
| `crates/` | New | Core CLI, engine, and platform drivers implementation. |
| `openspec/` | Modified | Specifications, designs, and tasks additions. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| System package non-determinism | Medium | Prefer native runtimes; use host managers only for fallbacks. |
| Windows path resolution complex | Medium | Run PowerShell/CMD runner compatibility integration tests early. |

## Rollback Plan
Since this bootstraps a greenfield repository, rollback consists of discarding uncommitted workspace files and git resetting to the empty repository head.

## Dependencies
- Native Rust compiler toolchain.
- Access to runtime package distribution URLs (Python, Node, Bun, Go, Rust).

## Success Criteria
- [ ] Statically linked `forge` binary compiles and runs on Windows, macOS, and Linux.
- [ ] Downloader successfully retrieves, verifies SHA-256, extracts, and caches all 5 runtimes.
- [ ] Command execution wraps child processes with correct env variables and PATH.
- [ ] `forge ai context` outputs valid, non-sensitive JSON environment maps.
