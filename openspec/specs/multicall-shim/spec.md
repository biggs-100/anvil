# Multicall Shim Specification

## Purpose

Intercepting environmental executable calls and routing them to target project-specific toolchains or global fallbacks.

## Requirements

### Requirement: Binary Resolution

The multicall shim binary MUST determine its execution context and select the target tool alias dynamically.

#### Scenario: Executable Name Interception
- GIVEN the shim binary is renamed to `node`
- WHEN the binary is executed using its current path
- THEN the system MUST resolve `node` as the target runtime toolchain alias

### Requirement: Cache Upward Search

The system MUST search the directory tree from the current working directory upwards for the project shim cache.

#### Scenario: Search Project Root
- GIVEN the current working directory is a deeply nested project subdirectory containing `.anvil/shims.cache` at the root
- WHEN the shim is executed
- THEN the system MUST traverse upwards and locate `.anvil/shims.cache`

### Requirement: Unix Process Replacement

On Unix platforms, the shim MUST execute the target tool using process replacement.

#### Scenario: Unix Execution
- GIVEN the platform is Unix and the target binary path is resolved
- WHEN forwarding the call
- THEN the system MUST invoke `execvp` to replace the current process image with the target tool

### Requirement: Windows Process Forwarding

On Windows platforms, the shim MUST forward stream data and signals to the target process.

#### Scenario: Windows Execution
- GIVEN the platform is Windows and the target binary path is resolved
- WHEN forwarding the call
- THEN the system MUST spawn the target tool, pipe standard streams, and exit with the tool's code

### Requirement: PATH Loop Prevention

The system MUST prevent self-referential execution loops.

#### Scenario: Strip Shim Directory from PATH
- GIVEN the shim directory is present in the `PATH` environment variable
- WHEN spawning the target binary process
- THEN the system MUST remove the shim directory from the target's `PATH` variable
