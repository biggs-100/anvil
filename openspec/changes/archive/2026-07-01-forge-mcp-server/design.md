# Design: Forge MCP Server

## Technical Approach

Add a `forge mcp` subcommand running an MCP (Model Context Protocol) stdio server in `forge-cli`, mirroring the existing `jsonrpc.rs` pattern. One new file (`mcp.rs`) with its own message types (no shared extraction — protocols differ in shape and lifecycle). Uses `McpExporter` for the `forge://context/active` resource and delegates all 6 tools to the existing `Engine` facade. Notifications subscribe to `EventBus` via a background task. Zero changes to `forge-core`.

## Architecture Decisions

| Decision | Options | Tradeoffs | Choice |
|----------|---------|-----------|--------|
| **Message types** | (a) Pull in MCP Rust crate, (b) Hand-roll serde structs | (a) Dep on unversioned external spec, extra weight; (b) ~80 LOC, full control, matches existing jsonrpc.rs style | **Hand-roll** — matches project pattern, avoids dependency risk |
| **Shared code with jsonrpc.rs** | (a) Extract stdio loop + write lock, (b) Keep separate | (a) Premature abstraction — only 2 consumers, protocols diverge further over time; (b) Duplicate ~30 LOC, each file self-contained | **Keep separate** — protocols differ in lifecycle (MCP has init handshake) and message shapes |
| **Notification strategy** | (a) Poll EventBus in main loop, (b) Spawn background subscriber task | (a) Simple but blocks on no-event periods; (b) Clean async pattern matching EventBus broadcast channel | **Background task** — EventBus uses `broadcast::Receiver`, perfect for tokio::spawn |
| **Concurrency model** | (a) One task per request (jsonrpc.rs style), (b) Sequential dispatch | (a) Handles concurrent requests but needs write lock; (b) Simple but blocks on slow operations | **One task per request** — consistent with jsonrpc.rs, proven for stdio transport |
| **Resource content** | (a) Inline McpExporter call per ReadResource, (b) Cache and refresh | (a) Always fresh, simple; (b) Stale risk, complexity not justified | **Inline** — McpExporter is fast and synchronous |

## Data Flow

```
stdin ──line──► Reader ──► Dispatch ──► Tool/Prompt Handler ──► Engine facade ──► stdout
                      │                    │     │         │
                      │                    │     │         └── McpExporter (read_resource)
                      │                    │     └──────────── Prompt templates
                      │                    └────────────────── Engine methods
                      │
Background task ──► EventBus.subscribe() ──► mcp_notification() ──► stdout
```

```
MCP Lifecycle:
  Client → initialize request      → Server responds with capabilities
  Client → list_tools request      → Server returns 6 tool definitions
  Client → tools/call (forge_run)  → Server delegates to Engine → returns result
  Client → resources/read          → Server calls McpExporter → returns context
  Client → shutdown notification   → Server exits cleanly
  EventBus event (state change)    → Background task → forge/state_changed notification
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-cli/src/mcp.rs` | Create | MCP server: message types, dispatch, handlers, notifications (~450 LOC) |
| `crates/forge-cli/src/main.rs` | Modify | Add `mod mcp;`, add `Mcp` variant to `Commands` enum, wire to `mcp::serve()` |

## Interfaces / Contracts

### MCP JSON-RPC Message Types (hand-rolled serde)

```rust
// ── Core envelope
struct McpRequest { jsonrpc: String, id: Option<Value>, method: String, params: Option<Value> }
struct McpResponse { jsonrpc: String, id: Option<Value>, result: Option<Value>, error: Option<McpError> }
struct McpError { code: i64, message: String, data: Option<Value> }
struct McpNotification { method: String, params: Value }    // no id → notification

// ── Initialize
struct InitializeParams { protocol_version: String, capabilities: ClientCapabilities, client_info: ClientInfo }
struct ServerCapabilities { resources: ResourcesCap, tools: ToolsCap, prompts: PromptsCap }
struct InitializeResult { protocol_version: String, capabilities: ServerCapabilities, server_info: ServerInfo }

// ── Resources
struct ReadResourceParams { uri: String }
struct ResourceContent { uri: String, mime_type: String, text: String }

// ── Tools
struct CallToolParams { name: String, arguments: Option<Value> }
struct ToolResultContent { mime_type: String, text: String }

// ── Prompts
struct GetPromptParams { name: String, arguments: Option<Value> }
struct PromptMessage { role: String, content: PromptContent }
```

### Tool → Engine Mapping

| MCP Tool | Engine Method | Input → Output |
|----------|--------------|----------------|
| forge_run | Engine.sync + RunOperation | cmd, args → exit_code, stdout, stderr |
| forge_shell | ShellOperation | — → session_id |
| forge_sync | Engine.sync | — → result |
| forge_plan | PlanOperation | — → plan summary |
| forge_explain | Engine.explain | runtime → explanation |
| forge_doctor | DiagnosticEngine.run | mode → diagnostic report |

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | MCP message serialization | Round-trip serde tests (same pattern as jsonrpc.rs tests) |
| Unit | Initialize handshake logic | Version negotiation, capability content |
| Unit | Prompt template rendering | Verify markdown output for forge:status/forge:diagnose/forge:explain |
| Integration | Protocol lifecycle | Pipe JSON to stdin, capture stdout, verify initialize → list tools → call tool flow |
| Integration | Tool error handling | Invalid cmd → error response, unknown tool → MethodNotFound |
| Integration | Resource reading | ReadResource forge://context/active → valid JSON with McpExporter content |
| E2E | Full session | Scripted stdin sequence through full lifecycle, verify all response types |

## Migration / Rollout

No migration required. Additive to forge-cli — existing `jsonrpc` and CLI commands unchanged. Single commit: add `mcp.rs`, wire `main.rs`. Rollback by reverting.

## Open Questions

- Notification batching: should multiple rapid state changes coalesce or each fire individually? Proposal: individual, simplest first.
- Prompt argument schemas: define JSON Schema for each prompt's optional arguments (e.g., forge:explain takes a runtime filter)? Start with args=none for v1.
