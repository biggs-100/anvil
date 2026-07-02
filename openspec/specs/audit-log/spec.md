# Audit Log Specification

## Purpose

The `forge audit` command SHALL display a history of download and verification operations from the existing journal (`journal.jsonl`), providing users with visibility into what forge has fetched, from where, and whether verification succeeded.

## Requirements

### Requirement: Audit Command

Forge MUST provide a `forge audit` subcommand that reads the operation journal and displays download/sync/install history.

- GIVEN the user runs `forge audit` after at least one sync or install operation
- THEN forge SHALL display a table with columns: timestamp, runtime, version, operation, URL, file size, SHA-256, and verification status

- GIVEN the user runs `forge audit` and the journal is empty or missing
- THEN forge SHALL display a message indicating no operations recorded
- AND SHALL exit with code 0

### Requirement: JSON Output

The `forge audit` command MUST support a `--json` flag that outputs machine-readable JSON.

- GIVEN the user runs `forge audit --json`
- THEN forge SHALL output a valid JSON array of operation records
- AND each record SHALL contain: timestamp, runtime, version, operation, url, size_bytes, sha256, verified

- GIVEN the user runs `forge audit --json` and the journal is empty
- THEN forge SHALL output `[]`

### Requirement: Verification Status

Each audit entry MUST display a verification status field indicating whether the artifact passed integrity checks.

- GIVEN an operation where the artifact hash matched its pin or lockfile entry
- THEN the verification status SHALL be `verified`

- GIVEN an operation where the artifact hash did not match
- THEN the verification status SHALL be `mismatch`

- GIVEN an operation where no hash verification was performed
- THEN the verification status SHALL be `not verified`

### Requirement: Source Tracking

Audit entries SHOULD include the source registry URL that provided the metadata or artifact.

- GIVEN an operation fetched from `https://registry.forge.sh`
- WHEN displayed in the audit log
- THEN the URL column SHALL include the full registry URL

### Requirement: SHA-256 Display

Audit entries MUST show the SHA-256 hash when it is available for the operation.

- GIVEN an operation where a SHA-256 was computed or recorded
- THEN the audit entry SHALL include the SHA-256 hex string

- GIVEN an operation where no SHA-256 was recorded (legacy entries)
- THEN the SHA-256 field SHALL show `—` or equivalent placeholder
