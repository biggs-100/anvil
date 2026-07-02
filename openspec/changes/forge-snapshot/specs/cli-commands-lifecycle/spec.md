# Delta for cli-commands-lifecycle

## ADDED Requirements

### Requirement: Snapshot and Restore CLI Commands

The system MUST add three lifecycle commands to the existing CLI table:

| Command | Input Arguments | Primary Side-Effect / Output | Target State |
|---------|----------------|------------------------------|--------------|
| `snapshot` | `--name`, `--description` | Creates `.forge/snapshots/{name}/` with config, lock, state, journal, and metadata | (Unchanged — snapshot is stateless) |
| `snapshot list` | None | Displays table of all snapshots sorted by creation date | (Unchanged) |
| `restore` | Snapshot name, `--dry-run` | Replaces forge.toml and forge.lock from snapshot, runs `forge up` | READY |

#### Scenario: Snapshot Command Invocation

- GIVEN the environment is in a READY or LOCKED state with forge.toml present
- WHEN `forge snapshot --name backup --description "Before risky change"` is executed
- THEN the system MUST create a snapshot directory with all required files
- AND output `Snapshot saved: backup`

#### Scenario: List Command Invocation

- GIVEN at least one snapshot exists in `.forge/snapshots/`
- WHEN `forge snapshot list` is executed
- THEN the system MUST display each snapshot's name, created_at, runtime_count, and description in a formatted table

#### Scenario: Restore Command Invocation

- GIVEN a snapshot named "baseline" exists
- WHEN `forge restore baseline` is executed
- THEN current forge.toml and forge.lock are backed up
- AND snapshot forge.toml and forge.lock replace the current files
- AND the system runs `forge up` to transition to READY state
