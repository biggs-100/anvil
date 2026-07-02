# Lockfile Generator Delta Specification

## Target

Modifies: [lockfile-generator](../../../../specs/lockfile-generator/spec.md)

## Purpose

Extend the lockfile metadata to record platform emulation details when native Windows ARM64 runtimes are missing and fallback execution is required.

## Added Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-LOCK-003 | The system MUST record emulation details under fallback conditions in `forge.lock` using 'requested', 'installed', and 'reason' attributes. | MUST |

### Requirement: Emulation Fallback Logging

#### Scenario: Log Windows ARM64 Emulation Fallback
- GIVEN a Windows ARM64 host requesting a native runtime that does not exist
- WHEN falling back to x86_64 emulation
- THEN the system MUST write `requested: "windows-arm64"`, `installed: "windows-x86_64"`, and `reason: "Native build unavailable"` to `forge.lock`.
