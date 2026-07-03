# Environment Activation Specification

## Purpose

Executing subprocesses with isolated environments, injecting local tool paths and configuration variables without modifying the host shell.

## Requirements

### Requirement: Subprocess Environment Injection

The system MUST inject tool path directories and environment variables from `anvil.env` into child subprocesses created by `anvil run` or `anvil shell`.

#### Scenario: Executing Command with Local Path Injection
- GIVEN `.anvil/runtimes/node` is cached and `anvil.env` contains `DB_USER=anvil`
- WHEN `anvil run node -v` is executed
- THEN the child process MUST run with the local Node path prepended to `PATH` and `DB_USER` in env

#### Scenario: Shell Activation
- GIVEN a parent shell
- WHEN `anvil shell` is executed
- THEN the system MUST launch a child shell inheriting the injected environment variables

#### Scenario: Host Isolation Preservation
- GIVEN a command is run via `anvil run` or `anvil shell`
- WHEN the subprocess completes and exits
- THEN the parent host shell's PATH and environment variables MUST remain unchanged
