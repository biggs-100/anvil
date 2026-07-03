# Runtime Engine Manifest Specification

## Purpose

Define the interface and behavior for finding, loading, and validating the manifest file (`anvil.toml`), and resolving the project root and lockfile paths.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-MNF-001 | The system MUST search parent directories upward from a starting directory to locate `anvil.toml`. | MUST |
| REQ-MNF-002 | The system MUST load and parse the content of `anvil.toml` into a strongly-typed `AnvilConfig`. | MUST |
| REQ-MNF-003 | The system MUST validate that the defined runtimes map names to valid version requirements. | MUST |
| REQ-MNF-004 | The system MUST resolve the project root directory as the parent of `anvil.toml`. | MUST |
| REQ-MNF-005 | The system MUST resolve the default lockfile path as `{project_root}/anvil.lock`. | MUST |

### Requirement: Manifest Loading & Resolution

#### Scenario: Manifest Found in Parent Directory
- GIVEN a start directory nested within a project
- WHEN `find_anvil_toml` is executed
- THEN the system MUST return the path to the root `anvil.toml`

#### Scenario: Invalid Manifest Format Failure
- GIVEN a `anvil.toml` with malformed TOML syntax
- WHEN `load_config` is executed
- THEN the system MUST return a parsing error
