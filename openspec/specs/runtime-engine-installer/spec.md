# Runtime Engine Installer Specification

## Purpose

Define download management, SHA-256 verification, multiple archive format extraction, and Zip Slip path traversal prevention.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-INS-001 | The system MUST download runtime packages asynchronously to the local cache. | MUST |
| REQ-INS-002 | The system MUST verify that the SHA-256 checksum of the downloaded file matches the expected hash. | MUST |
| REQ-INS-003 | The system MUST clean up temporary/partial files if a download or extraction fails. | MUST |
| REQ-INS-004 | The system MUST support extracting ZIP, TarGz, and TarXz archive formats. | MUST |
| REQ-INS-005 | The system MUST prevent path traversal (Zip Slip) by rejecting archive paths that resolve outside the destination directory. | MUST |

### Requirement: Secure Download and Extraction

#### Scenario: Hash Verification Success
- GIVEN a downloaded archive and expected hash `e3b0c442...`
- WHEN checksum validation runs and hashes match
- THEN the system MUST accept the archive and proceed to extraction

#### Scenario: Path Traversal Attempt Blocked
- GIVEN an archive entry with path `../../etc/passwd`
- WHEN extraction processes the entry
- THEN the system MUST reject the extraction and abort with a security error
