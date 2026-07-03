# Runtime Manager Delta Specification

## Target

Modifies: [runtime-manager](../../../../specs/runtime-manager/spec.md)

## Purpose

Trigger shims cache regeneration upon successful toolchain operations.

## Added Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-MGR-004 | The system MUST regenerate `.anvil/shims.cache` upon successful completion of any runtime installation, update, or package lock modification. | MUST |

### Requirement: Cache Regeneration Trigger

#### Scenario: Successful Install Triggers Cache Regeneration
- GIVEN a successful runtime toolchain download and extraction
- WHEN the installation task completes successfully
- THEN the system MUST regenerate the `.anvil/shims.cache` file with the updated binary paths
