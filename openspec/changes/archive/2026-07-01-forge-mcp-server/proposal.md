# Proposal: Forge MCP Server

## Intent

Expose Forge's full engine capabilities via the Model Context Protocol (MCP), enabling AI agents to inspect project context, execute commands, run diagnostics, and receive state change notifications through a standardized protocol interface.

## Scope

### In Scope
- `forge mcp` subcommand in forge-cli (stdio transport)
- Resource `forge://context/active` — full project context via McpExporter
- Tools: `forge_run`, `forge_shell`, `forge_sync`, `forge_plan`, `forge_explain`, `forge_doctor`
- Prompts: `forge:status`, `forge:diagnose`, `forge:explain`
- Notifications: `forge/state_changed`, `forge/error`, `forge/warning`
- MCP JSON-RPC message types (Initialize, ListResources, ReadResource, ListTools, CallTool, ListPrompts, GetPrompt, Notifications)

### Out of Scope
- SSE/HTTP transport (stdio-only for v1)
- Remote MCP server registration or discovery
- Dynamic resource registration from plugins
- Non-stdio auth or session management

## Capabilities

### New Capabilities
- `mcp-server`: Full MCP stdio server exposing resources, tools, prompts, and notifications over the Model Context Protocol

### Modified Capabilities
- None (additive to forge-cli only; zero changes to forge-core)

## Approach

Build on the same pattern as `forge jsonrpc` — a dedicated module `crates/forge-cli/src/mcp.rs` implementing the MCP protocol on top of the existing Engine facade. Reuses `McpExporter` from `forge-core` for the `forge://context/active` resource. Maps MCP tools to existing Engine operations. Dispatches notifications via the EventBus. No changes to forge-core's frozen surface.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-cli/src/main.rs` | Modified | Add `Mcp` variant to Commands enum |
| `crates/forge-cli/src/mcp.rs` | New | MCP server module (~500 lines) |
| `crates/forge-core/src/context/mod.rs` | Unchanged | Existing McpExporter reused |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| MCP spec evolution | Medium | Thin protocol layer — update message types per latest spec |
| Stdio transport reliability | Low | Same pattern as jsonrpc — proven in production |
| Zero core changes constraint | Low | MCP layer is pure additive to forge-cli |

## Rollback Plan

Revert the `forge mcp` subcommand addition in `main.rs` and delete `mcp.rs`. No core surface is touched, so rollback is a single-commit revert.

## Dependencies

- MCP spec (modelcontextprotocol/specification) — validate against latest release
- `forge-core` as-is (McpExporter, Engine, EventBus, DiagnosticEngine)

## Success Criteria

- [ ] `forge mcp` starts, responds to Initialize, and exposes `forge://context/active` resource
- [ ] All 6 tools return correct results for valid inputs and errors for invalid inputs
- [ ] All 3 prompts render valid MCP prompt messages
- [ ] Notifications fire on state change, error, and warning conditions
- [ ] Zero changes to `crates/forge-core/` (frozen surface preserved)
