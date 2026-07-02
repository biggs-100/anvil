# Runtime Engine Cache Specification

## Purpose

Define directory layout rules, cache state detection, shims map generation, and registry of local toolchains.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-CCH-001 | The system MUST store cached runtimes under a standardized path structure: `~/.forge/runtimes/{name}/{version}/extracted`. | MUST |
| REQ-CCH-002 | The system MUST skip download and extraction if the target extraction directory exists and is non-empty. | MUST |
| REQ-CCH-003 | The system MUST scan extracted runtime directories to find directories containing executable binaries. | MUST |
| REQ-CCH-004 | The system MUST write a signature-verified `.forge/shims.cache` mapping binary names to absolute paths. | MUST |

### Requirement: Cache Operations

#### Scenario: Skip Installation on Cache Hit
- GIVEN an extracted directory containing executable files exists
- WHEN `install_runtimes` is run for that runtime
- THEN the system MUST skip download and extraction and return success

#### Scenario: Shims Cache Generation
- GIVEN a list of installed runtimes (e.g. node, python)
- WHEN `regenerate_shims_cache` is invoked
- THEN the system MUST scan binary paths and write them to `.forge/shims.cache` with a SHA-256 header signature
