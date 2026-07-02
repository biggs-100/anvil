# Runtime Manager Delta Specification

## Purpose

This delta spec modifies the existing Runtime Manager specification to integrate toolchain downloading and extraction with staging folders and atomic commit promotion hooks.

## Added Requirements

### Requirement: Transactional Stage and Promotion
The Runtime Manager MUST delegate download and extraction tasks to the isolated staging directory prior to promotion.

#### Scenario: Toolchain Staging Before Promotion
- GIVEN Python is missing from the local cache
- WHEN Python is requested for download
- THEN the system MUST download, verify, and extract Python to `.forge/staging/<operation_id>/python` and delay commit until final transaction verification.

#### Scenario: Parallel Downloads Stage in Isolation
- GIVEN Bun and Go are requested in parallel
- WHEN download starts
- THEN the system MUST download and extract both runtimes into isolated subdirectory paths within the same staging parent folder.

## Modified Requirements

### Requirement: Cache Regeneration Trigger
- **Modified**: The shims cache regeneration is now decoupled from individual runtime installation and MUST only execute after a successful commit/promotion hook has completed.

#### Scenario: Successful Promotion Triggers Shim Regeneration
- GIVEN a successful transaction commit of multiple runtimes
- WHEN the promotion phase completes successfully
- THEN the system MUST regenerate `.forge/shims.cache` with the active paths.
