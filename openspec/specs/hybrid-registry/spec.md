# Hybrid Registry Specification

## Purpose

Coordinate runtime resolution using an offline-first metadata cache and a fallback remote registry.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-REG-001 | The registry MUST consult `.forge/metadata_cache.toml` first for version and coordinate metadata. | MUST |
| REQ-REG-002 | The registry MUST fail resolution immediately if an exact uncached version is requested while offline. | MUST |
| REQ-REG-003 | The registry MUST resolve loose version ranges (e.g. "^20") to the latest matching cached version if offline. | MUST |

### Requirement: Exact Version Offline Constraint

#### Scenario: Offline Exact Version Missing
- GIVEN the network is offline and Node "20.11.0" is not in `.forge/metadata_cache.toml`
- WHEN resolving Node version "20.11.0"
- THEN the system MUST fail with a network/offline resolution error.

### Requirement: Range Resolution Offline Compatibility

#### Scenario: Offline Range Matching Cache
- GIVEN the network is offline and Node versions "20.10.0" and "20.9.0" are cached
- WHEN resolving Node version range "^20"
- THEN the system MUST resolve to "20.10.0" using cached metadata.
