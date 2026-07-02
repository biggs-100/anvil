# Atomic Transactions Specification

## Purpose

Define the transactional directory staging-to-commit rules, atomic folder rename promotion hooks, and fallback rollback error cleanup strategies to guarantee zero partial runtime installations on disk.

## Requirements

### Requirement: Isolation in Staging Folder
The system MUST download, verify, and extract all toolchains inside a temporary staging folder (`.forge/staging/<operation_id>`) isolated from the active runtime directory.

### Requirement: Atomic Promotion Commit Hook
The system MUST promote all staged toolchains in a single transaction to `.forge/runtimes/` using atomic directory renames once all checks pass.
- The staging and backup directories MUST reside on the same filesystem partition as target caches.
- If a target runtime directory already exists, the system MUST back up the original directory to `.forge/backup/<runtime>` prior to promotion.
- On Windows, the promotion phase MUST implement a retry policy with exponential backoff to handle transient file locks.

### Requirement: Transactional Rollback
On any promotion failure, the system MUST rollback all target directories to their original state and discard the staging directory.
- The backup directories MUST be restored to `.forge/runtimes/`.
- Newly promoted directories MUST be deleted.

#### Scenario: Successful Transaction Commit
- GIVEN a sync operation staging Python and Node in `.forge/staging/op123`
- WHEN validation is successful
- THEN the system MUST backup existing Python and Node, atomically rename staging directories to `.forge/runtimes/python` and `.forge/runtimes/node`, and delete backups on success.

#### Scenario: Staged Validation Failure Prevents Promotion
- GIVEN Python is staged successfully but Node fails verification in staging
- WHEN the promotion phase is reached
- THEN the system MUST abort promotion, leave `.forge/runtimes` untouched, delete the staging folder, and report execution failure.

#### Scenario: Promotion Rollback on Mid-Phase Failure
- GIVEN Python is promoted successfully but Node promotion fails due to a locked destination directory
- WHEN the transaction rollback is triggered
- THEN the system MUST delete the newly promoted Python directory and restore Python's original folder from `.forge/backup/python`.
