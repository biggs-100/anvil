# Lockfile Generator Specification

## Purpose

Generating and parsing a deterministic lockfile (`forge.lock`) to ensure consistent environments across machines.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-LOCK-001 | The system MUST generate `forge.lock` in a standard format containing exact toolchain metadata (version, platform, architecture, url, size, sha256). | MUST |
| REQ-LOCK-002 | The system MUST resolve and synchronize runtimes using the exact versions and checksums specified in `forge.lock`. | MUST |
| REQ-LOCK-003 | The system MUST record emulation details under fallback conditions in `forge.lock` using 'requested', 'installed', and 'reason' attributes. | MUST |

### Requirement: Lockfile Format and Contents

#### Scenario: Generating New Lockfile
- GIVEN Node and Python are configured for installation
- WHEN `forge lock` or compilation runs
- THEN the system MUST write `forge.lock` with deterministic sorted keys containing version, platform, architecture, url, size, and sha256 hash

#### Scenario: Synchronizing from Existing Lockfile
- GIVEN an existing `forge.lock` in the workspace
- WHEN runtimes are requested
- THEN the system MUST fetch the exact versions and checksums specified in `forge.lock`

### Requirement: Emulation Fallback Logging

#### Scenario: Log Windows ARM64 Emulation Fallback
- GIVEN a Windows ARM64 host requesting a native runtime that does not exist
- WHEN falling back to x86_64 emulation
- THEN the system MUST write `requested: "windows-arm64"`, `installed: "windows-x86_64"`, and `reason: "Native build unavailable"` to `forge.lock`.
