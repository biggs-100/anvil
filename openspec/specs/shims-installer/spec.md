# Shims Installer Specification

## Purpose

Installing workspace shims in the user's home directory and validating system environment configurations.

## Requirements

### Requirement: Setup Installation

The `forge setup` command MUST create the shims directory and copy the shim binary under all supported toolchain aliases.

#### Scenario: Copy Shim Aliases
- GIVEN a system with `forge` installed
- WHEN running `forge setup`
- THEN the system MUST create `~/.forge/bin` if missing and copy the shim executable as `node`, `python`, `bun`, `go`, `cargo`, and `rust`

### Requirement: Uninstall Cleanup

The `forge setup --uninstall` command MUST remove all installed shims and clean up the shims directory.

#### Scenario: Remove Shims Directory
- GIVEN active shims inside `~/.forge/bin`
- WHEN running `forge setup --uninstall`
- THEN the system MUST delete all shim files and remove the `~/.forge/bin` directory

### Requirement: Doctor Path Validation

The `forge doctor` command MUST inspect the system PATH configuration and warn if shims are inactive.

#### Scenario: Missing PATH Warning
- GIVEN the directory `~/.forge/bin` is not present in the current `PATH` environment variable
- WHEN running `forge doctor`
- THEN the system MUST print a warning alert detailing instructions to add the directory to `PATH`
