# Anvil Runtime Registry Specification (ARRS)

## Purpose

Define an open, forward-compatible TOML-based format for describing runtime toolchain artifacts, their mirrors, and a registry index. ARRS eliminates hardcoded download URLs by enabling registry-driven resolution.

## Requirements

| ID | Description | Strength |
|---|---|---|
| REQ-ARRS-001 | The system MUST define a `metadata.toml` schema per the structure below. | MUST |
| REQ-ARRS-002 | The system MUST support multiple platforms and architectures per toolchain version. | MUST |
| REQ-ARRS-003 | The system SHOULD include a `mirrors.json` with fallback download URLs. | SHOULD |
| REQ-ARRS-004 | Every ARRS file MUST be forward-compatible — tools MUST ignore unknown fields. | MUST |
| REQ-ARRS-005 | The registry MUST expose an `index.toml` listing all available toolchains with latest versions. | MUST |
| REQ-ARRS-006 | The system MUST resolve a download artifact for a specific platform + arch from metadata. | MUST |

### Requirement: metadata.toml Schema

Each toolchain version publishes a `metadata.toml` at `{name}/{version}/metadata.toml` containing:

```toml
name = "python"
version = "3.13.0"
license = "PSF"
homepage = "https://python.org"
artifacts = [
  { platform = "windows", arch = "x86_64", url = "...", size = 12345, sha256 = "abc..." },
]
dependencies = []  # optional runtime dependencies
```

Unknown fields (e.g. `description`, `release_date`) MUST be silently ignored.

#### Scenario: Parse metadata.toml
- GIVEN a valid `metadata.toml` for Python 3.13.0
- WHEN the system parses it
- THEN it MUST extract `name`, `version`, `license`, `homepage`, `artifacts`, and `dependencies`

#### Scenario: Ignore Unknown Fields
- GIVEN a `metadata.toml` with an extra `description` field
- WHEN the system parses it
- THEN it MUST NOT error and MUST return the known fields

### Requirement: Platform Artifact Resolution

#### Scenario: Resolve Artifact for Specific Platform
- GIVEN a `metadata.toml` with artifacts for `windows/x86_64` and `linux/x86_64`
- WHEN resolving an artifact for `windows/x86_64`
- THEN the system MUST return the matching artifact entry with URL, size, and sha256

### Requirement: Mirror Configuration

#### Scenario: Fallback to Mirror
- GIVEN a `mirrors.json` with two mirror URLs
- WHEN the primary artifact URL is unreachable
- THEN the system SHOULD attempt download from the first mirror, then the second

### Requirement: Registry Index

An `index.toml` at the registry root lists available toolchains:

```toml
[toolchains.python]
latest = "3.13.0"
versions = ["3.13.0", "3.12.0"]

[toolchains.node]
latest = "22.0.0"
versions = ["22.0.0", "20.11.0"]
```

#### Scenario: List Available Versions from Index
- GIVEN an `index.toml` listing Python 3.13.0 and 3.12.0
- WHEN listing available versions for Python
- THEN the system MUST return `["3.12.0", "3.13.0"]`
