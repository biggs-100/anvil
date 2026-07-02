# forge-snapshot Specification

## Purpose

Save and restore full environment state as named snapshots for rollback, bug reproduction, and runtime version experimentation.

## Requirements

### Requirement: Snapshot Directory Structure

Each snapshot MUST be stored in `.forge/snapshots/{name}/` with these files:

| File | Source |
|------|--------|
| `forge.toml` | Copied verbatim |
| `forge.lock` | Copied verbatim |
| `state.json` | Lifecycle state captured at snapshot time |
| `journal.jsonl` | Last 100 journal events |
| `snapshot.json` | Generated metadata |

The system MUST NOT include `.forge/runtimes/` binaries or `.forge/cache/` files.

#### Scenario: All Required Files Present

- GIVEN an environment at READY state with config, lockfile, and journal events
- WHEN `forge snapshot` is executed
- THEN the snapshot MUST contain all five files and forge.toml/forge.lock MUST be byte-identical copies

### Requirement: Snapshot Metadata

`snapshot.json` MUST contain `created_at` (ISO 8601), `forge_version` (semver), `runtime_count`, and `name`. It SHOULD contain `description` when provided.

#### Scenario: Metadata Written Correctly

- GIVEN `forge snapshot --name backup --description "Before Node upgrade"` is executed
- THEN snapshot.json includes name="backup", description, created_at, forge_version, and runtime_count

### Requirement: `forge snapshot` Command

The system MUST auto-generate a UTC timestamp name. It MUST exit 1 if `forge.toml` is missing. It SHOULD accept `--name` and `--description`. Output MUST print `Snapshot saved: {name}` with the path.

#### Scenario: Default Snapshot Created

- GIVEN a project with forge.toml
- WHEN `forge snapshot` is executed
- THEN `.forge/snapshots/<timestamp>/` is created and output confirms the name

#### Scenario: Named Snapshot

- GIVEN forge.toml exists
- WHEN `forge snapshot --name pre-upgrade --description "Before Node 20"` is executed
- THEN the snapshot is saved to `.forge/snapshots/pre-upgrade/` with metadata populated

#### Scenario: Missing forge.toml

- GIVEN no forge.toml exists in the current directory
- WHEN `forge snapshot` is executed
- THEN the system exits with code 1 and prints an error

### Requirement: `forge snapshot list` Command

The system MUST list all snapshots as a formatted table (name, created_at, runtime_count, description), sorted by created_at descending.

#### Scenario: Empty Snapshots

- GIVEN `.forge/snapshots/` is empty or absent
- WHEN `forge snapshot list` is executed
- THEN the system prints "No snapshots found"

#### Scenario: List Multiple Snapshots

- GIVEN three snapshots (alpha, beta, gamma) created in order
- WHEN `forge snapshot list` is executed
- THEN all three appear in reverse chronological order with all four columns

### Requirement: `forge restore` Command

The system MUST restore forge.toml and forge.lock from a named snapshot, backing up current files first. It MUST exit 1 if the snapshot is missing. After restoring files, it MUST run `forge up`. It SHOULD support `--dry-run`.

#### Scenario: Successful Restore

- GIVEN snapshot "pre-upgrade" exists with config/lock
- WHEN `forge restore pre-upgrade` is executed
- THEN current files are backed up, snapshot files replace them, and `forge up` re-syncs runtimes

#### Scenario: Non-Existent Snapshot

- GIVEN no snapshot named "missing" exists
- WHEN `forge restore missing` is executed
- THEN the system exits 1 with an error and modifies NO files

#### Scenario: Dry-Run Preview

- GIVEN snapshot "pre-upgrade" exists
- WHEN `forge restore pre-upgrade --dry-run` is executed
- THEN the system displays what would be restored but changes NO files
