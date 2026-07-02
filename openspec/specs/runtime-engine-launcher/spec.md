# Runtime Engine Launcher Specification

## Purpose

Define requirements and scenarios for spawning processes, managing shell execution within custom environments, and capturing exit codes.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-LNC-001 | The system MUST spawn child processes inside the custom runtime-activated environment. | MUST |
| REQ-LNC-002 | The system MUST wait for and capture the exit code of spawned processes. | MUST |
| REQ-LNC-003 | The system MUST spawn the host platform's default shell when an interactive environment is requested. | MUST |

### Requirement: Process Execution

#### Scenario: Command Spawns and Returns Exit Code
- GIVEN a custom environment with Node active
- WHEN a command `node -v` is spawned
- THEN the system MUST execute the command, capture exit code `0`, and return it

#### Scenario: Interactive Shell Spawns
- GIVEN a shell request on Windows
- WHEN `spawn_shell_in_env` is executed
- THEN the system MUST spawn `powershell.exe` (or `COMSPEC` target) loaded with active runtime paths
