# Observability Which Specification

## Purpose

Providing command-line visibility and resolution diagnostics for active project toolchains.

## Requirements

### Requirement: Resolution Diagnostic Info

The `anvil which <runtime>` command MUST output details on how a runtime name is resolved.

#### Scenario: Display Resolved Toolchain Information
- GIVEN an active project using a cached `node` toolchain
- WHEN running `anvil which node`
- THEN the system MUST print the target binary path, version, resolving source (e.g. workspace cache), and the project context path

### Requirement: Missing Resolution Diagnostics

The system MUST report failures and return error exit codes when a toolchain is not found.

#### Scenario: Toolchain Not Found
- GIVEN a request for a runtime that is not installed or configured in the workspace or system
- WHEN running `anvil which missing-tool`
- THEN the system MUST print a diagnostic error message and exit with a non-zero status code
