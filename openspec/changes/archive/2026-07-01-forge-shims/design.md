# Design: anvil-shims

## Technical Approach

Introduce a fast, dependency-free Rust binary `anvil-shim` to intercept tool calls (e.g., `node`, `python`, `bun`, `go`, `cargo`, `rust`) and route them to the active workspace-specific runtime path parsed from `.anvil/shims.cache`. If the local cache is absent or the tool is unmapped, it falls back to the host system binary, ensuring loop recursion prevention by filtering out the shim directory. 

## Architecture Decisions

| Option | Tradeoff | Decision |
|---|---|---|
| **Cache Format**: TOML vs simple ini-like key-value lines | TOML is robust but requires parser dependencies; key-value lines are parsed with zero dependencies in <1ms. | Use custom line-by-line key-value parsing (`key = value`) for `.anvil/shims.cache`. |
| **Unix Exec**: `std::process::Command` vs `execvp` process replacement | Spawning a child keeps the shim process running (extra PID/memory); `execvp` replaces the process image completely. | Use `CommandExt::exec()` on Unix to replace the process image. |
| **Windows Exec**: Spawning and forwarding | Windows does not natively support process image replacement. | Spawn child process and forward stdin, stdout, stderr, signals, and exit status. |
| **PATH Loop Prevention**: Filter current binary vs filter shim folder | Filtering just the tool name may match system-wide shims; filtering the shim directory avoids recursion. | Remove `current_exe()` parent directory from `PATH` prior to invoking host fallback. |

## Data Flow

```
[Command Invocation] (e.g. node)
        │
        ▼
[Determine current_exe() name]
        │
        ▼
[Traverse parents upwards for .anvil/shims.cache]
        │
   ┌────┴───────────────────────────┐
   │ Cache Found & Maps Tool?       │
   └────┬───────────────────────┬───┘
        │ Yes                   │ No
        ▼                       ▼
[Run Local Target Binary]   [Filter shim dir from PATH]
                                │
                                ▼
                            [Search host PATH for tool]
                                │
                           ┌────┴─────────────────┐
                           │ Found globally?      │
                           └────┬─────────────┬───┘
                                │ Yes         │ No
                                ▼             ▼
                            [Exec Tool]   [Print Help Warning & Exit 1]
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-shim/Cargo.toml` | Create | Minimal Cargo manifest targeting fast compile and startup. |
| `crates/anvil-shim/src/main.rs` | Create | Entry point for intercepting executing names, parsing cache, and forwarding. |
| `crates/anvil-core/src/lib.rs` | Modify | Add `.gitignore` updater logic and shim config helpers. |
| `crates/anvil-cli/src/main.rs` | Modify | Integrate `anvil setup`, `anvil doctor`, and `anvil which`. |

## Interfaces / Contracts

### Cache Schema (`.anvil/shims.cache`)
Located at project root's `.anvil/shims.cache`. Parsed line-by-line.
```ini
# anvil-shims-cache-v1
# generated_at: 2026-07-01T07:45:21-05:00
# version_signature: 1a2b3c4d
node = C:\Users\USER\.forge\runtimes\node\20.10.0\extracted\bin\node.exe
python = C:\Users\USER\.forge\runtimes\python\3.11.0\extracted\bin\python.exe
```

### Gitignore Incremental Updater
`crates/anvil-core` will append these entries to the project's `.gitignore` if not present:
```gitignore
# Anvil artifacts
.anvil/shims.cache
.anvil/state.json
```

### Warning Messages
If a tool is not found locally or globally:
`"Python/Node/etc is not available. Anvil did not find a local config or a global install. Run 'anvil init' or 'anvil install <tool>'."`

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Cache Parsing | Mock key-value file contents and assert expected paths are parsed. |
| Unit | Parent Traversal | Verify traversal terminates safely at path root. |
| Integration | Loop Prevention | Run shim with target executable absent from PATH except shim directory; assert fallback fails safely. |
| Integration | Argument Forwarding | Assert arguments and exit codes propagate correctly. |

## Migration / Rollout

No data migration required. 
- **Rollout**: `anvil setup` copies `anvil-shim` into `~/.anvil/bin` under different runtime aliases.
- **Rollback**: Remove target files from `~/.anvil/bin` or clear folder.
