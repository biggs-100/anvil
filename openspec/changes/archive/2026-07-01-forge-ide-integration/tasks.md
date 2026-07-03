# Tasks: Anvil IDE Integration

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~600-750 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr-default |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

_Solo dev, not in production — single PR with size exception pre-authorized._

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | VS Code extension (MCP client + UI) | PR 1 | All VS Code files; base = main |
| 2 | Neovim plugin (MCP client + UI) | PR 1 | All Neovim files; independent of VS Code |
| 3 | README documentation | PR 1 | Both READMEs in same PR |

## Phase 1: VS Code MCP Client

- [x] 1.1 Create `extensions/vscode/src/mcpClient.ts` — spawn `anvil mcp` via child_process, line-buffered JSON-RPC parser, pending request Map, notification dispatch
- [x] 1.2 Implement `connect()` (init handshake), `sendRequest()` (promise-based), `disconnect()` (SIGTERM → 3s → SIGKILL), `getState()`

## Phase 2: VS Code UI

- [x] 2.1 Create `extensions/vscode/package.json` — 4 commands, activation on `anvil.toml`, @types/vscode devDep
- [x] 2.2 Create `extensions/vscode/tsconfig.json` — strict, Node 18+ target
- [x] 2.3 Create `extensions/vscode/.vscodeignore` — VSIX packaging filter
- [x] 2.4 Create `extensions/vscode/src/statusBar.ts` — StatusBarItem with 3 states: connected (health), disconnected (red), not-found
- [x] 2.5 Create `extensions/vscode/src/forgePanel.ts` — WebviewPanel rendering markdown from prompts/get
- [x] 2.6 Create `extensions/vscode/src/commands.ts` — 4 handlers mapping to prompts/get and tools/call
- [x] 2.7 Create `extensions/vscode/src/extension.ts` — activation: spawn mcp, register commands+status+panel+diagnostics, deactivate cleanup

## Phase 3: Neovim MCP Client

- [x] 3.1 Create `extensions/neovim/lua/forge/mcp.lua` — vim.fn.jobstart MCP client: line-buffered stdout, pending request map, notification → vim.diagnostic

## Phase 4: Neovim UI

- [x] 4.1 Create `extensions/neovim/lua/forge/init.lua` — module entry: 4 user commands, BufRead autocmd for anvil.toml, VimLeave cleanup
- [x] 4.2 Create `extensions/neovim/lua/forge/ui.lua` — float windows (status/explain), quickfix (doctor), terminal (run), Telescope picker
- [x] 4.3 Create `extensions/neovim/lua/forge/health.lua` — :checkhealth forge: verify binary, Neovim version, job state

## Phase 5: Documentation

- [x] 5.1 Create `extensions/vscode/README.md` — setup, 4 commands, requirements (VS Code 1.82+, Node 18+)
- [x] 5.2 Create `extensions/neovim/README.md` — lazy.nvim setup, 4 commands, requirements (Neovim 0.9+)
