# anvil-snapshot Specification

## Purpose

Save and restore full environment state as named snapshots for rollback, bug reproduction, and runtime version experimentation.

## Requirements

### Requirement: Snapshot Directory Structure

Each snapshot MUST be stored in `.anvil/snapshots/{name}/` with these files:

| File | Source |
|------|--------|
| `anvil.toml` | Copied verbatim |
| `anvil.lock` | Copied verbatim |
| `state.json` | Lifecycle state captured at snapshot time |
| `journal.jsonl` | Last 100 journal events |
| `snapshot.json` | Generated metadata |

The system MUST NOT include `.anvil/runtimes/` binaries or `.anvil/cache/` files.

#### Scenario: All Required Files Present

- GIVEN an environment at READY state with config, lockfile, and journal events
- WHEN `anvil snapshot` is executed
- THEN the snapshot MUST contain all five files and anvil.toml/anvil.lock MUST be byte-identical copies

### Requirement: Snapshot Metadata

`snapshot.json` MUST contain `created_at` (ISO 8601), `anvil_version` (semver), `runtime_count`, and `name`. It SHOULD contain `description` when provided.

#### Scenario: Metadata Written Correctly

- GIVEN `anvil snapshot --name backup --description "Before Node upgrade"` is executed
- THEN snapshot.json includes name="backup", description, created_at, anvil_version, and runtime_count

### Requirement: `anvil snapshot` Command

The system MUST auto-generate a UTC timestamp name. It MUST exit 1 if `anvil.toml` is missing. It SHOULD accept `--name` and `--description`. Output MUST print `Snapshot saved: {name}` with the path.

#### Scenario: Default Snapshot Created

- GIVEN a project with anvil.toml
- WHEN `anvil snapshot` is executed
- THEN `.anvil/snapshots/<timestamp>/` is created and output confirms the name

#### Scenario: Named Snapshot

- GIVEN anvil.toml exists
- WHEN `anvil snapshot --name pre-upgrade --description "Before Node 20"` is executed
- THEN the snapshot is saved to `.anvil/snapshots/pre-upgrade/` with metadata populated

#### Scenario: Missing anvil.toml

- GIVEN no anvil.toml exists in the current directory
- WHEN `anvil snapshot` is executed
- THEN the system exits with code 1 and prints an error

### Requirement: `anvil snapshot list` Command

The system MUST list all snapshots as a formatted table (name, created_at, runtime_count, description), sorted by created_at descending.

#### Scenario: Empty Snapshots

- GIVEN `.anvil/snapshots/` is empty or absent
- WHEN `anvil snapshot list` is executed
- THEN the system prints "No snapshots found"

#### Scenario: List Multiple Snapshots

- GIVEN three snapshots (alpha, beta, gamma) created in order
- WHEN `anvil snapshot list` is executed
- THEN all three appear in reverse chronological order with all four columns

### Requirement: `anvil restore` Command

The system MUST restore anvil.toml and anvil.lock from a named snapshot, backing up current files first. It MUST exit 1 if the snapshot is missing. After restoring files, it MUST run `anvil up`. It SHOULD support `--dry-run`.

#### Scenario: Successful Restore

- GIVEN snapshot "pre-upgrade" exists with config/lock
- WHEN `anvil restore pre-upgrade` is executed
- THEN current files are backed up, snapshot files replace them, and `anvil up` re-syncs runtimes

#### Scenario: Non-Existent Snapshot

- GIVEN no snapshot named "missing" exists
- WHEN `anvil restore missing` is executed
- THEN the system exits 1 with an error and modifies NO files

#### Scenario: Dry-Run Preview

- GIVEN snapshot "pre-upgrade" exists
- WHEN `anvil restore pre-upgrade --dry-run` is executed
- THEN the system displays what would be restored but changes NO files
