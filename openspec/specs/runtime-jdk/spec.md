# Runtime JDK Specification

## Purpose

Define the JDK toolchain provider contract for downloading, verifying, and resolving JDK releases from the Adoptium API. Provides Java compilation tooling (`java`, `javac`, `jar`) for projects using `forge.toml`.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-JDK-001 | Providers MUST download pre-built JDK binaries from `api.adoptium.net` | MUST |
| REQ-JDK-002 | Providers MUST verify downloaded archives against the published SHA-256 checksum | MUST |
| REQ-JDK-003 | Providers MUST expose the runtime name `jdk` for use in `forge.toml` `[runtimes]` | MUST |
| REQ-JDK-004 | Providers MUST support LTS versions (e.g., 17, 21) and current feature releases | MUST |
| REQ-JDK-005 | Providers MUST support Windows x86_64, MacOS x86_64 + aarch64, Linux x86_64 + aarch64 | MUST |
| REQ-JDK-006 | Providers MUST resolve version strings like `"21.0.2"`, `"17.0.9"` against the Adoptium API | MUST |
| REQ-JDK-007 | Providers MUST emit FRRS-compatible `RegistryEntry` metadata for registry persistence | MUST |

### Requirement: JDK Version Resolution

#### Scenario: Resolve JDK 21 for macos-aarch64
- GIVEN a JDK provider configured on macos-aarch64
- WHEN requested to resolve version `"21.0.2"`
- THEN the provider MUST return the exact version `21.0.2`, a download URL matching `api.adoptium.net/v3/binary/latest/21/ga/mac/aarch64/jdk/hotspot/normal/eclipse`, the archive size, and a valid SHA-256 hash

#### Scenario: Resolve JDK 17 LTS
- GIVEN a JDK provider configured on linux-x86_64
- WHEN requested to resolve version `"17.0.9"`
- THEN the provider MUST return version `17.0.9` with a valid Adoptium binary URL for linux-x86_64

#### Scenario: Unsupported Platform Requested
- GIVEN a JDK provider configured on an unsupported platform
- WHEN requested to resolve any version
- THEN the provider MUST raise a `PlatformNotSupported` error without attempting a download
