# Tasks: Forge MCP Server

## Review Workload Forecast

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

Estimated changed lines: 500–650. If accepting single PR, maintainer must approve size-exception. Recommended split: PR 1 (types+loop+CLI), PR 2 (handlers+notifications), PR 3 (tests).

## Phase 1: MCP Protocol Types

- [x] 1.1 Define `McpRequest`, `McpResponse`, `McpError`, `McpNotification` serde structs in `crates/forge-cli/src/mcp.rs`
- [x] 1.2 Define `InitializeParams`, `InitializeResult`, `ServerCapabilities`
- [x] 1.3 Define `ReadResourceParams`, `ResourceContent`, `CallToolParams`, `ToolResultContent`
- [x] 1.4 Define `GetPromptParams`, `PromptMessage`
- [x] 1.5 Round-trip serde tests for all message types

## Phase 2: Core Server Loop

- [x] 2.1 Implement `serve()` — BufReader stdin loop with write_lock (jsonrpc.rs pattern)
- [x] 2.2 Parse JSON into `McpRequest`/`McpNotification`; dispatch by method
- [x] 2.3 Implement initialize handshake with state machine (uninitialized → initialized)
- [x] 2.4 Write `McpResponse` to stdout; unknown method → `MethodNotFound`
- [x] 2.5 Handle shutdown notification — break loop and exit cleanly

## Phase 3: Resource Handler

- [x] 3.1 Implement `handle_list_resources()` — return `forge://context/active`
- [x] 3.2 Implement `handle_read_resource()` — call `McpExporter`, return JSON

## Phase 4: Tool Handlers

- [x] 4.1 Implement `handle_list_tools()` — all 6 tools with JSON Schema inputs
- [x] 4.2 Implement `forge_run` — Engine.sync + RunOperation, return exit_code/stdout/stderr
- [x] 4.3 Implement `forge_shell` — ShellOperation, return session_id
- [x] 4.4 Implement `forge_sync` — Engine.sync, return result
- [x] 4.5 Implement `forge_plan` — PlanOperation, return plan summary
- [x] 4.6 Implement `forge_explain` — Engine.explain, return runtime explanation
- [x] 4.7 Implement `forge_doctor` — DiagnosticEngine.run, return diagnostic report

## Phase 5: Prompt Handlers

- [x] 5.1 Implement `handle_list_prompts()` — return 3 prompt definitions
- [x] 5.2 Implement forge:status — return markdown status overview
- [x] 5.3 Implement forge:diagnose — return markdown health diagnostics
- [x] 5.4 Implement forge:explain — return markdown config explanation

## Phase 6: Notifications

- [x] 6.1 Spawn background task subscribing to `EventBus` broadcast receiver
- [x] 6.2 Map events → `forge/state_changed` with old/new state
- [x] 6.3 Map tool errors → `forge/error` with operation + error details
- [x] 6.4 Map warnings → `forge/warning` with finding + severity
- [x] 6.5 Write notifications to stdout as `McpNotification` JSON via write_lock

## Phase 7: CLI Integration

- [x] 7.1 Add `mod mcp;` to `crates/forge-cli/src/main.rs`
- [x] 7.2 Add `Mcp` variant to `Commands` enum with about description
- [x] 7.3 Dispatch `Commands::Mcp` → `mcp::serve()` in `run_cli()`

## Phase 8: Testing

- [x] 8.1 Integration test: initialize handshake → capabilities response
- [x] 8.2 Integration test: list_tools returns 6 definitions
- [ ] 8.3 Integration test: forge_run valid cmd → exit_code + stdout + stderr
- [ ] 8.4 Integration test: forge_run invalid cmd → non-zero exit
- [ ] 8.5 Integration test: forge_doctor quick mode → report
- [x] 8.6 Integration test: unknown method → MethodNotFound (-32601)
- [x] 8.7 Integration test: ReadResource forge://context/active → JSON
- [ ] 8.8 Integration test: full lifecycle through shutdown notification
