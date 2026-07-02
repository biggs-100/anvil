# Delta for environment-lifecycle-rfc

## ADDED Requirements

### Requirement: Snapshot Preconditions and Behavior

Taking a snapshot MUST be available from any stable state (INITIALIZED, RESOLVED, LOCKED, SYNCED, READY). A snapshot MUST NOT change the current lifecycle state — it is a stateless checkpoint. Restoring from a snapshot MUST replace forge.toml and forge.lock, then execute `forge up` to transition through RESOLVED → LOCKED → SYNCED → READY.

#### Scenario: Snapshot From READY State

- GIVEN the environment is in READY state
- WHEN `forge snapshot` is executed
- THEN the environment remains in READY state after the snapshot completes

#### Scenario: Restore From Snapshot State Transition

- GIVEN a snapshot exists
- AND the current environment is in READY state with different config/lock
- WHEN `forge restore <snapshot-name>` is executed
- THEN current files are backed up, snapshot files replace config/lock
- AND the system transitions through RESOLVED, LOCKED, SYNCED, and back to READY via `forge up`

### Requirement: Invalid Restore Preconditions

Restore MUST NOT proceed if the environment is in a BROKEN or DIRTY state unless `--force` is provided (SHOULD support in future). If the target snapshot directory is missing, the system MUST reject the operation before any file modifications.

#### Scenario: Restore Blocked on Missing Snapshot

- GIVEN no snapshot named "nonexistent" exists
- WHEN `forge restore nonexistent` is executed
- THEN the system MUST print an error and exit with code 1
- AND MUST NOT modify any files or transition state
