# Delta for cli-commands-lifecycle

## ADDED Requirements

### Requirement: Snapshot and Restore CLI Commands

The system MUST add three lifecycle commands to the existing CLI table:

| Command | Input Arguments | Primary Side-Effect / Output | Target State |
|---------|----------------|------------------------------|--------------|
| `snapshot` | `--name`, `--description` | Creates `.anvil/snapshots/{name}/` with config, lock, state, journal, and metadata | (Unchanged — snapshot is stateless) |
| `snapshot list` | None | Displays table of all snapshots sorted by creation date | (Unchanged) |
| `restore` | Snapshot name, `--dry-run` | Replaces anvil.toml and anvil.lock from snapshot, runs `anvil up` | READY |

#### Scenario: Snapshot Command Invocation

- GIVEN the environment is in a READY or LOCKED state with anvil.toml present
- WHEN `anvil snapshot --name backup --description "Before risky change"` is executed
- THEN the system MUST create a snapshot directory with all required files
- AND output `Snapshot saved: backup`

#### Scenario: List Command Invocation

- GIVEN at least one snapshot exists in `.anvil/snapshots/`
- WHEN `anvil snapshot list` is executed
- THEN the system MUST display each snapshot's name, created_at, runtime_count, and description in a formatted table

#### Scenario: Restore Command Invocation

- GIVEN a snapshot named "baseline" exists
- WHEN `anvil restore baseline` is executed
- THEN current anvil.toml and anvil.lock are backed up
- AND snapshot anvil.toml and anvil.lock replace the current files
- AND the system runs `anvil up` to transition to READY state
