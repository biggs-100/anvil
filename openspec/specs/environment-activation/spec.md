# Environment Activation Specification

## Purpose

Executing subprocesses with isolated environments, injecting local tool paths and configuration variables without modifying the host shell.

## Requirements

### Requirement: Subprocess Environment Injection

The system MUST inject tool path directories and environment variables from `forge.env` into child subprocesses created by `forge run` or `forge shell`.

#### Scenario: Executing Command with Local Path Injection
- GIVEN `.forge/runtimes/node` is cached and `forge.env` contains `DB_USER=forge`
- WHEN `forge run node -v` is executed
- THEN the child process MUST run with the local Node path prepended to `PATH` and `DB_USER` in env

#### Scenario: Shell Activation
- GIVEN a parent shell
- WHEN `forge shell` is executed
- THEN the system MUST launch a child shell inheriting the injected environment variables

#### Scenario: Host Isolation Preservation
- GIVEN a command is run via `forge run` or `forge shell`
- WHEN the subprocess completes and exits
- THEN the parent host shell's PATH and environment variables MUST remain unchanged
