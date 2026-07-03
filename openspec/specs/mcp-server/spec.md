# MCP Server Specification

## Purpose

Expose Anvil's engine capabilities via the Model Context Protocol (MCP), enabling AI agents to inspect project context, execute commands, run diagnostics, and receive state notifications over a standardized JSON-RPC 2.0 interface via stdin/stdout transport.

## Requirements

### Requirement: Protocol Lifecycle

The system MUST implement the MCP lifecycle: initialize handshake with version negotiation, initialized state, message exchange, and clean shutdown.

#### Scenario: Successful initialization
- GIVEN a client connects via stdin
- WHEN the client sends an initialize request with a supported protocol version
- THEN the server responds with its capabilities and the agreed protocol version
- AND the server transitions to the initialized state

#### Scenario: Graceful shutdown
- GIVEN the server is initialized
- WHEN the server receives a shutdown notification
- THEN the server stops processing new requests and exits cleanly

### Requirement: Resource anvil://context/active

The system MUST expose a resource `anvil://context/active` that returns the full AnvilContext serialized via McpExporter.

#### Scenario: Read active context
- GIVEN the server is initialized
- WHEN a client sends a ReadResource request for URI `anvil://context/active`
- THEN the server returns a resource content with MIME type `application/json`
- AND the content contains the complete serialized AnvilContext

### Requirement: Tool Commands

The system MUST provide these tools, each delegating to the Engine facade:

| Tool | Input | Output |
|------|-------|--------|
| anvil_run | cmd, args | exit_code, stdout, stderr |
| anvil_shell | — | session_id |
| anvil_sync | — | result |
| anvil_plan | — | plan summary |
| anvil_explain | runtime | explanation |
| anvil_doctor | mode | diagnostic report |

#### Scenario: anvil_run executes a command
- GIVEN the anvil environment is ready
- WHEN a client calls anvil_run with valid cmd and args
- THEN the server returns exit_code (zero for success), captured stdout, and captured stderr

#### Scenario: anvil_run returns error on invalid command
- GIVEN the anvil environment is ready
- WHEN a client calls anvil_run with a nonexistent cmd
- THEN the server returns a non-zero exit_code and an error message in stderr

#### Scenario: anvil_doctor runs diagnostics
- GIVEN the server is initialized
- WHEN a client calls anvil_doctor with mode "quick"
- THEN the server runs diagnostics via DiagnosticEngine and returns the report

#### Scenario: anvil_shell spawns subshell
- GIVEN the anvil environment is ready
- WHEN a client calls anvil_shell
- THEN the server returns a unique session_id identifying the subshell

### Requirement: Prompts

The system MUST provide these prompt templates, returning markdown-formatted messages:

| Prompt | Description |
|--------|-------------|
| anvil:status | Current environment state summary |
| anvil:diagnose | Diagnose project issues and health |
| anvil:explain | Explain runtime configuration in detail |

#### Scenario: anvil:status returns environment overview
- GIVEN the server is initialized
- WHEN a client requests the anvil:status prompt
- THEN the server returns a markdown message summarizing the environment state

### Requirement: Notifications

The system MUST emit these notifications via the EventBus when events occur:

| Notification | Payload |
|-------------|---------|
| anvil/state_changed | {old_state, new_state} |
| anvil/error | {operation, error} |
| anvil/warning | {finding, severity} |

#### Scenario: State change fires notification
- GIVEN the engine transitions between lifecycle states
- WHEN the transition occurs
- THEN the server emits a anvil/state_changed notification with old_state and new_state

#### Scenario: Operation error fires notification
- GIVEN a tool operation fails
- WHEN the error is detected
- THEN the server emits a anvil/error notification with the operation name and error details

### Requirement: Error Handling

The system MUST use standard JSON-RPC 2.0 error codes and gracefully handle unknown methods.

#### Scenario: Unknown method returns MethodNotFound
- GIVEN the server is initialized
- WHEN a client sends a request for an unsupported method
- THEN the server responds with JSON-RPC error code -32601 (MethodNotFound)
