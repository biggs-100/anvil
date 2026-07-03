# CLI Commands Lifecycle Specification

## Purpose

Define the input and output behavior of the 13 environment lifecycle commands.

## Requirements

### Requirement: Unified CLI Command Input/Output Contracts
All CLI commands MUST interact with the lifecycle state machine and return consistent exit codes (0 for success, non-zero for failure) and formatted output.

| Command | Input Arguments | Primary Side-Effect / Output | Target State |
|---|---|---|---|
| `init` | Path / Template | Creates `anvil.toml` configuration | INITIALIZED |
| `resolve` | Configuration | Version resolution manifest generated | RESOLVED |
| `lock` | Resolve manifest | Writes `anvil.lock` with SHA-256 hashes | LOCKED |
| `sync` | Lockfile, `--force` | Downloads, verifies, extracts, commits runtimes | READY |
| `up` | Config, Lockfile | Runs resolve -> lock -> sync sequentially | READY |
| `run` | Command + args | Runs target command with loaded environment | ACTIVE |
| `shell` | Shell type | Launches interactive subshell | ACTIVE |
| `clean` | `--all` / Runtime | Deletes `.anvil/runtimes` / staging folders | INITIALIZED |
| `gc` | `--dry-run` | Deletes orphaned/unused cached runtime folders | (Unchanged) |
| `status` | None | Emits JSON representation of current state | (Unchanged) |
| `inspect` | Runtime name | Prints detailed metadata, paths, env variables | (Unchanged) |
| `repair` | None | Runs 5-step pipeline to fix corrupted environments | READY |
| `plan` | None | Generates and prints `SyncPlan` / `RepairPlan` | (Unchanged) |

#### Scenario: Running sync from LOCKED State
- GIVEN the current state is LOCKED
- WHEN `anvil sync` is executed
- THEN the system MUST download, verify, extract, promote the runtimes, and update the state to READY.

#### Scenario: Shell Activation
- GIVEN the current state is READY
- WHEN `anvil shell` is executed
- THEN the system MUST launch a shell environment prepend the paths to the shims, and transition the active environment state to ACTIVE.

#### Scenario: Run Command Execution
- GIVEN the current state is READY
- WHEN `anvil run python --version` is executed
- THEN the system MUST execute the command within the environment, output python's version, and exit with status 0.
