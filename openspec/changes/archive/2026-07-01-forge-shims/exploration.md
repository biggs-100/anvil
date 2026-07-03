## Exploration: anvil-shims

### Current State
Currently, `forge` resolves and installs runtimes (Node, Python, Bun, Go, Rust) into a local cache directory (`~/.anvil/runtimes`). To execute commands within the sandboxed environment, users must prefix their command with `anvil run <cmd>` or spawn a new shell using `anvil shell`. There is no mechanism to run runtimes directly (e.g. calling `node app.js`) while dynamically routing to the correct workspace-configured version without explicit wrapper CLI commands.

### Affected Areas
- `crates/anvil-core/src/lib.rs` — Needs to support writing a compiled/pre-resolved shim cache (e.g. `.anvil/shims.cache`) containing the exact paths to the extracted binary directories during the lockfile update and runtime extraction phase.
- `crates/anvil-cli/src/main.rs` — Needs to support a new command (e.g., `anvil shims install` or automatic hooks inside existing `anvil run` / `anvil lock` commands) to set up and manage the global shim directory.
- `crates/anvil-shim` (New Crate) — A lightweight, independent Rust crate to compile the `anvil-shim` executable. Keeping it in a separate crate allows compiling it with minimal dependencies (no Tokio, no Reqwest, no heavy Serde) and special compiler optimization flags for rapid startup.

### Approaches

1. **Global Shims (`~/.anvil/shims`) with Contextual Resolver**
   - **Description**: Add `~/.anvil/shims` to the user's system `PATH` once. This directory contains copies/links of a multi-call `anvil-shim` executable named after the supported runtimes (e.g., `node`, `python`, `bun`, `go`, `rust`). When executed, the shim determines its runtime context from its own file name, traverses upward to find the nearest `anvil.toml`/`anvil.lock`, resolves the runtime version, and executes the cached binary. If outside a project, it scans `PATH` (skipping the shim directory) to forward execution to the system-installed runtime.
   - **Pros**:
     - Transparent, "always-on" experience across all terminals. No manual environment activation or shell wrapping needed.
     - Dynamic version resolution updates instantly when switching directories.
   - **Cons**:
     - Must be extremely fast since it intercepts commands globally.
     - Fallback mechanism must be robust to prevent infinite recursion loops when calling system binaries.
   - **Effort**: Medium

2. **Project-Local Shims (`.anvil/shims`)**
   - **Description**: Each project contains a local `.anvil/shims` directory. When running `anvil run` or activating the environment, `.anvil/shims` is prepended to the active terminal's `PATH`. These shims can be hardcoded script wrappers or symlinks pointing directly to the resolved binary in the cache for that specific project.
   - **Pros**:
     - Simplifies the shim code: no upward directory traversal or dynamic version resolution needed at runtime, as the shim is pre-configured for the project's exact version.
     - No risk of intercepting commands outside the project or causing infinite path-recursion loops.
   - **Cons**:
     - Poor user experience: requires manual environment activation (`source .anvil/bin/activate` or `anvil shell`) per terminal session.
     - Creating symlinks on Windows requires administrative privileges by default, requiring fallback copies or complex workarounds.
   - **Effort**: Low

### Recommendation
We recommend **Approach 1 (Global Shims with Contextual Resolver)**. This aligns with modern version managers (e.g., `asdf`, `nodenv`, `pyenv`, `cargo/rustup`) to provide a fully transparent developer experience. 

To achieve the **<5ms overhead** performance target:
1. **Multicall Binary**: A single, stripped `anvil-shim` Rust executable (no async, no heavy crates). The runtime name is determined via `std::env::current_exe()`.
2. **Pre-Resolved Cache**: When `anvil lock` or `anvil run` is executed, we generate a small, flat cache file `.anvil/shims.cache` in the workspace root. The shim scans upward for this file, reads it, and executes the cached path immediately, avoiding lockfile/TOML parsing or semver evaluation on the critical execution path.
3. **Unix Process Replacement**: On Unix, the shim MUST use `std::os::unix::process::CommandExt::exec` (`execvp`) to replace the shim process with the target runtime process, introducing zero post-launch resource overhead.
4. **Windows Process Forwarding**: On Windows, the shim spawns the target child process and forwards signals, stdin, stdout, and stderr, exiting with the child's exit code.

### Risks
- **PATH Loop Recursion**: If the shim falls back to the system version of a tool but fails to correctly filter out its own shim directory from the `PATH` environment variable, it will invoke itself recursively, causing stack/resource exhaustion.
- **Windows Executable Locking**: In Windows, active executables are locked. If a runtime binary or the shim itself needs updating while running, it can cause permission errors.
- **Windows Command/Shell Differences**: Argument parsing differences (e.g., cmd vs PowerShell vs bash on Windows) can lead to subtle shell-escape bugs when forwarding arguments on Windows.

### Ready for Proposal
Yes. The orchestrator should present this technical analysis to the user, highlighting the recommended Global Shim architecture and the caching strategy to meet the <5ms execution overhead target.
