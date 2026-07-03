# Runtime LLVM Specification

## Purpose

Define the LLVM/Clang toolchain provider contract for downloading, verifying, and resolving pre-built LLVM releases from GitHub. Acts as the source of C/C++ compilation tooling for projects using `anvil.toml`.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-LLVM-001 | Providers MUST download pre-built LLVM/Clang binaries from `github.com/llvm/llvm-project/releases` | MUST |
| REQ-LLVM-002 | Providers MUST verify downloaded archives against the published SHA-256 checksum | MUST |
| REQ-LLVM-003 | Providers MUST expose the runtime name `llvm` for use in `anvil.toml` `[runtimes]` | MUST |
| REQ-LLVM-004 | Providers SHOULD include `clangd` and `lld` alongside `clang`/`clang++` | SHOULD |
| REQ-LLVM-005 | Providers MUST support Windows x86_64, MacOS x86_64 + aarch64, Linux x86_64 + aarch64 | MUST |
| REQ-LLVM-006 | Providers MUST accept version strings in semver format (e.g., `"18.1.0"`) | MUST |
| REQ-LLVM-007 | Providers MUST emit ARRS-compatible `RegistryEntry` metadata for registry persistence | MUST |

### Requirement: LLVM Version Resolution

#### Scenario: Resolve LLVM 18.1.0 for linux-x86_64
- GIVEN an LLVM provider configured on linux-x86_64
- WHEN requested to resolve version `"18.1.0"`
- THEN the provider MUST return the exact version `18.1.0`, a download URL matching `github.com/llvm/llvm-project/releases/download/llvmorg-18.1.0`, the archive size, and a valid SHA-256 hash

#### Scenario: Platform Binary Not Available
- GIVEN an LLVM provider configured on an unsupported platform (e.g., 32-bit arm)
- WHEN requested to resolve any version
- THEN the provider MUST raise a `PlatformNotSupported` error without attempting a download

### Requirement: Integrity Verification

#### Scenario: Checksum Mismatch Detected
- GIVEN a downloaded LLVM archive
- WHEN its SHA-256 hash does not match the expected value from the release metadata
- THEN the provider MUST discard the archive and raise a `ChecksumMismatch` error
