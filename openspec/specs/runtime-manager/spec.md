# Runtime Manager Specification

## Purpose

Resolving, downloading, checksum verifying, extracting, and caching precompiled runtime packages.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-MGR-001 | The system MUST manage toolchains (Python, Node, Bun, Go, Rust) by checking the local cache, downloading when missing, verifying SHA-256 hashes, and extracting packages. | MUST |
| REQ-MGR-002 | The system MUST execute downloads, hash verifications, and extractions of multiple runtimes concurrently using asynchronous tasks. | MUST |
| REQ-MGR-003 | The system MUST delegate resolution, URL mapping, and pre-installation verification to specific runtime providers. | MUST |
| REQ-MGR-004 | The system MUST regenerate `.anvil/shims.cache` only after a successful transaction commit/promotion has completed, not after individual runtime installations. | MUST |
| REQ-MGR-005 | The system MUST delegate download and extraction tasks to an isolated staging directory (`.anvil/staging/<operation_id>/`) and delay commit until final transaction verification. | MUST |

### Requirement: Runtime Toolchain Management

#### Scenario: Toolchain Cache Hit
- GIVEN Python is cached in `.anvil/runtimes` matching the lockfile SHA-256
- WHEN a task requests Python execution
- THEN the system MUST run Python from the cache without downloading

#### Scenario: Download and Extract
- GIVEN Node is missing from `.anvil/runtimes`
- WHEN Node is requested for execution
- THEN the system MUST download Node, verify its SHA-256, extract it, and cache it

#### Scenario: Hash Verification Failure
- GIVEN a downloaded Bun package has a SHA-256 mismatch
- WHEN verification is run
- THEN the system MUST delete the downloaded package and abort execution with an error

### Requirement: Parallel Orchestration

#### Scenario: Parallel Download and Extraction
- GIVEN Node and Python are both uncached and requested for a task run
- WHEN execution starts
- THEN the system MUST download, verify, and extract both runtimes in parallel.

#### Scenario: Parallel Action Failure Propagation
- GIVEN Bun and Go are downloaded in parallel
- WHEN Bun download fails or verification fails
- THEN the system MUST abort all active parallel downloads and report the failure.

### Requirement: Transactional Stage and Promotion

#### Scenario: Toolchain Staging Before Promotion
- GIVEN Python is missing from the local cache
- WHEN Python is requested for download
- THEN the system MUST download, verify, and extract Python to `.anvil/staging/<operation_id>/python` and delay commit until final transaction verification.

#### Scenario: Parallel Downloads Stage in Isolation
- GIVEN Bun and Go are requested in parallel
- WHEN download starts
- THEN the system MUST download and extract both runtimes into isolated subdirectory paths within the same staging parent folder.

### Requirement: Cache Regeneration Trigger (Modified)

The shims cache regeneration is now decoupled from individual runtime installation and MUST only execute after a successful commit/promotion hook has completed.

#### Scenario: Successful Promotion Triggers Shim Regeneration
- GIVEN a successful transaction commit of multiple runtimes
- WHEN the promotion phase completes successfully
- THEN the system MUST regenerate `.anvil/shims.cache` with the active paths.
