# Design: Anvil Bootstrap

## Technical Approach

Forge is implemented as a Rust workspace to guarantee sub-millisecond startups, direct system interaction, and a single-binary distribution. We will split the codebase into three main crates: a CLI wrapper (`anvil-cli`), a core engine (`anvil-core`) for config/lockfile resolution and downloads, and system fallback drivers (`anvil-drivers`). Downloads are executed concurrently using `tokio` and verified via SHA-256 before extraction.

## Architecture Decisions

| Decision Area | Option | Tradeoff | Decision |
| :--- | :--- | :--- | :--- |
| **Storage Architecture** | Project-local `.anvil/runtimes` vs Global `~/.anvil/runtimes` | Local duplicates binaries across repositories; Global saves space but requires path mapping/symlinks. | Cache runtimes in `~/.anvil/runtimes/`, run directly or symlink. |
| **Activation Model** | Shell Hooks (`cd` hooks) vs Subprocess Wrapping | Shell hooks require complex shell-specific setups (Zsh, Powershell); wrapping is robust and shell-agnostic. | Use Subprocess Wrapping via `anvil run <cmd>` and `anvil shell`. |
| **Crate Boundaries** | Monolith Crate vs Workspace Division | Monolith compiles slightly faster; division separates CLI parsing, engine logic, and OS drivers cleanly. | Partition into `anvil-cli`, `anvil-core`, and `anvil-drivers`. |

## Data Flow

```
[anvil.toml/env] ──> [anvil-cli (clap)] ──> [anvil-core (Engine)]
                                                    │
                                           (Check Cache / Download)
                                                    │
                                           ┌────────┴────────┐
                                    [~/.anvil/runtimes]  [anvil-drivers (Fallback)]
                                           │
                                  (Prepend PATH & Env)
                                           │
                                    [std::process::Command]
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Create | Root workspace Cargo manifest. |
| `crates/anvil-cli/Cargo.toml` | Create | CLI package manifest (uses `clap`, `serde_json`). |
| `crates/anvil-cli/src/main.rs` | Create | Parses CLI arguments and handles human/AI UI formatting. |
| `crates/anvil-core/Cargo.toml` | Create | Core manifest (uses `tokio`, `reqwest`, `serde`, `toml`). |
| `crates/anvil-core/src/lib.rs` | Create | Handles configuration, lockfiles, runtime downloads, and execution. |
| `crates/anvil-drivers/Cargo.toml` | Create | Drivers crate manifest. |
| `crates/anvil-drivers/src/lib.rs` | Create | Fallback platform package managers execution. |

## Interfaces / Contracts

### Manifest (`anvil.toml`)
```toml
[runtimes]
node = "20.11.0"
python = "3.12.0"
```

### Lockfile (`anvil.lock`)
```toml
[[runtime]]
name = "node"
version = "20.11.0"
platform = "windows"
arch = "x86_64"
url = "https://nodejs.org/dist/v20.11.0/node-v20.11.0-win-x64.zip"
size = 31234567
sha256 = "d41d8cd98f00b204e9800998ecf8427e"
```

### Context Schema (`anvil ai context`)
```json
{
  "project_type": "rust_workspace",
  "active_runtimes": {
    "node": "20.11.0",
    "python": "3.12.0"
  },
  "env_vars": {
    "DB_USER": "anvil",
    "API_KEY": "[REDACTED]"
  }
}
```

### Doctor Schema (`anvil ai doctor`)
```json
{
  "status": "unhealthy",
  "issues": [
    {
      "id": "missing_runtime",
      "severity": "critical",
      "tool": "node",
      "message": "Node.js v20.11.0 is required but not installed.",
      "remediation": "anvil run"
    }
  ]
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | TOML parser, path/env generators, masking logic | Mock input configs, verify expected structs and redacted output. |
| Integration | Subprocess injection, shell spawns | Execute commands (e.g. `echo $PATH`), verify correct binary is invoked. |
| Mock Downloader | HTTP download, checksum verification | Local server stub returning test blobs with matching/mismatched SHA-256. |

## Migration / Rollout

Green-field bootstrap project. No data migration is required. Rollback is executed by doing `git reset --hard` to restore the repository to its clean state.

## Open Questions

- Should `anvil ai context` recursively traverse parent directories for `anvil.toml` configurations?
