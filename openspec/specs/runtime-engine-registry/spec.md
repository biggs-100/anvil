# Runtime Engine Registry Specification

## Purpose

Define registry coordination, registry cache loading, and metadata compatibility queries for mapping runtime requirements to actual packages.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-REG-001 | The system MUST coordinate metadata queries across default internal and local cached registries. | MUST |
| REQ-REG-002 | The system MUST load local registry metadata cache from a file (e.g. `.anvil/metadata_cache.toml`). | MUST |
| REQ-REG-003 | The system MUST match entries based on name, normalized platform, and normalized architecture. | MUST |
| REQ-REG-004 | The system MUST sort matched entries by version in descending order to prefer newer releases. | MUST |

### Requirement: Registry Operations

#### Scenario: Load Local Registry File
- GIVEN a custom `.anvil/metadata_cache.toml` exists
- WHEN the registry coordinator is initialized
- THEN the system MUST load metadata entries from this file

#### Scenario: No Version Found Error
- GIVEN a requirement "^99.0.0" that does not match any entry
- WHEN resolution is queried
- THEN the system MUST return a clear error stating no matching version exists
