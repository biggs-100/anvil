# Design: Anvil IDE Integration

## Technical Approach

Thin MCP clients embedded in VS Code (TypeScript) and Neovim (Lua). Each IDE spawns `anvil mcp` as a stdio subprocess on activation, sends JSON-RPC 2.0 line-delimited messages, and renders responses in native UI surfaces. Zero anvil business logic runs inside the IDE layer — every capability comes from the anvil engine through the MCP protocol defined in `crates/anvil-cli/src/mcp.rs`.

Commands map to two MCP mechanisms: **prompts/get** (markdown output → rich display) and **tools/call** (JSON output → structured panels/lists). Notifications (`forge/warning`, `forge/error`) push to native diagnostic collections.

## Architecture Decisions

| Option | Tradeoffs | Decision |
|--------|-----------|----------|
| **Shared MCP client lib** | Cross-language bridge for TS↔Lua; would need N-API or IPC helper. Gains: unified bug fixes. Cost: complexity exceeds benefit for 4 commands. | **Independent impls** — each IDE owns its ~80-line MCP client |
| **VS Code: TreeView vs Webview** | TreeView: native feel, async tree updates. Webview: full HTML rendering for markdown from prompts. | **WebviewPanel** for status/explain (rich markdown from prompt results) |
| **Neovim: Telescope vs custom float** | Telescope: built-in fuzzy find, less boilerplate. Custom float: no dependency on telescope.nvim. | **Both** — custom float for core commands (zero-dep), Telescope picker as optional extension |
| **Neovim job lifecycle** | Single job (simpler state) vs multi-job pool | **Single job** — one `anvil mcp` process, cached job ID, restart callback on crash |
| **Async MCP response routing** | Per-request callback map vs event-emitter pattern | **Pending requests map** (`Map<id, {resolve,reject}>`) — standard JSON-RPC pattern, matches `mcp.rs` dispatch |

## Data Flow

```
IDE Extension                  anvil mcp (subprocess)
    │                                │
    │  spawn child_process / jobstart│
    │════════════════════════════════>│
    │                                │
    │  MCP initialize (request)      │
    │───────────────────────────────>│
    │< initialize (response) ────────│
    │                                │
    │  notifications/initialized ───>│
    │                                │
    │  ──── User executes command ── │
    │                                │
    │  tools/call or prompts/get ───>│── dispatch → anvil engine
    │< result (JSON/markdown) ───────│
    │                                │
    │  ──── Background ───────────── │
    │< forge/warning notification ───│── EventBus subscriber
    │< forge/error notification ──── │
    │                                │
    │  shutdown (notification) ─────>│
    │  close stdin ──────────────────>│── EOF → loop exit
    │  wait 3s, SIGKILL if alive ───>│
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `extensions/vscode/package.json` | Create | Extension manifest: 4 commands, activation on `anvil.toml`, `@types/vscode` devDependency |
| `extensions/vscode/tsconfig.json` | Create | Strict TS config targeting Node 18+ for extension host |
| `extensions/vscode/.vscodeignore` | Create | VSIX packaging filter |
| `extensions/vscode/src/extension.ts` | Create | Activation: spawn `anvil mcp`, register commands, create status bar + panel + diagnostics |
| `extensions/vscode/src/mcpClient.ts` | Create | MCP stdio client: spawn, line-buffered read, pending request map, notification dispatch |
| `extensions/vscode/src/commands.ts` | Create | Command handlers mapping to `prompts/get` (status, diagnose) and `tools/call` (explain, run) |
| `extensions/vscode/src/forgePanel.ts` | Create | WebviewPanel for status/explain markdown rendering |
| `extensions/vscode/src/statusBar.ts` | Create | StatusBarItem: connected (health score), disconnected (red), not found |
| `extensions/neovim/lua/forge/init.lua` | Create | Module entry: user commands, autocmd on anvil.toml, cleanup on VimLeave |
| `extensions/neovim/lua/forge/mcp.lua` | Create | MCP client via `vim.fn.jobstart`: line-buffered stdout, pending request map, notification → vim.diagnostic |
| `extensions/neovim/lua/forge/ui.lua` | Create | Floating windows (status, explain), quickfix list (doctor), terminal buffer (run), Telescope picker |
| `extensions/neovim/lua/forge/health.lua` | Create | `:checkhealth forge` — verify anvil binary, Neovim version, running state |

## Interfaces / Contracts

### MCP Method Mapping

| IDE Command | MCP Method | MCP Name | Params | Output |
|-------------|-----------|----------|--------|--------|
| `Forge: Show Status` / `:ForgeStatus` | `prompts/get` | `forge:status` | `{}` | Markdown → Webview / Float |
| `Forge: Diagnose` / `:ForgeDoctor` | `prompts/get` / `tools/call` | `forge:diagnose` / `forge_doctor` | `{}` / `{"mode": "quick"}` | Markdown→Problems / JSON→Quickfix |
| `Forge: Explain` / `:ForgeExplain` | `tools/call` | `forge_explain` | `{"runtime": "..."}` | JSON → Webview / Float |
| `Forge: Run` / `:ForgeRun` | `tools/call` | `forge_run` | `{"cmd":"...","args":[...]}` | JSON → Output channel / Terminal |

### MCP Client Interface (both IDEs)

```typescript
// Pseudocode — applies to both TS and Lua
interface McpClient {
  connect(): Promise<void>         // spawn + initialize handshake
  sendRequest(method, params): Promise<JsonRpcResponse>
  onNotification(method, handler): void
  disconnect(): void               // shutdown + kill subprocess
  getState(): 'connecting' | 'ready' | 'disconnected' | 'error'
}
```

### Notification Contract

```typescript
// forge/warning
{ method: "anvil/warning", params: { finding: string, severity: "WARNING" } }
// forge/error
{ method: "anvil/error", params: { operation: string, error: string } }
// forge/state_changed
{ method: "anvil/state_changed", params: { old_state: string, new_state: string } }
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | MCP client (line parsing, request routing) | Mock stdin/stdout, test with real JSON-RPC fixtures from `mcp.rs` tests |
| Integration | Full command flow via `anvil mcp` binary | Integration test spawns `anvil mcp` from a tmpdir with `anvil.toml`, sends initialize + prompt/tool, asserts response shape |
| E2E | VS Code extension activation | Manual: F5 launch in VS Code with `anvil.toml` workspace |
| E2E | Neovim plugin commands | Manual: `nvim +Lazy load forge` in project dir, run `:ForgeStatus` |
| Health | `:checkhealth forge` | Automated via `checkhealth` framework — verifies binary, PATH, job state |

## Migration / Rollout

No migration required. All files are new additions. Rollout per proposal rollback plan: uninstall extension / remove plugin spec.

## Open Questions

- [ ] VS Code: `forge:diagnose` (prompt returns markdown) vs `forge_doctor` (tool returns JSON) — which better populates the problems panel? Spec maps to `forge:diagnose` prompt; implementation may switch to `forge_doctor` tool if structured entries are needed for `DiagnosticCollection`.
- [ ] Neovim: should `:ForgeDoctor` populate quickfix (as specified) or `vim.diagnostic`? Quickfix is more discoverable, but diagnostics auto-show with signs.
- [ ] Telescope picker: what data source to list? Runtimes from `forge_explain` or config entries?
