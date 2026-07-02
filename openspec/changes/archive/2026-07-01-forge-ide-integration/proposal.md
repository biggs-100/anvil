# Proposal: Forge IDE Integration

## Intent

Expose Forge's MCP capabilities inside VS Code and Neovim. Each IDE spawns `forge mcp` as a subprocess and communicates via MCP protocol — no business logic in the IDE layer, just UI rendering over MCP transport.

## Scope

### In Scope
- VS Code extension: status bar indicator, webview panel, 4 commands, diagnostic markers from forge notifications, VSIX packaging
- Neovim plugin: floating windows for status/explain, quickfix for diagnostics, Telescope integration, auto-show diagnostics, lazy.nvim packaging

### Out of Scope
- JetBrains, Zed, or other IDEs (deferred)
- Standalone GUI (Phase 13)
- Cloud Sync (Phase 14)
- IDE-side caching or offline mode

## Capabilities

### New Capabilities
- `ide-vscode`: VS Code extension that connects to `forge mcp` via stdio and renders MCP responses in native VS Code UI (status bar, webview panel, commands, problems panel)
- `ide-neovim`: Neovim Lua plugin that connects to `forge mcp` via stdio and renders MCP responses in native Neovim UI (floats, quickfix, Telescope picker)

### Modified Capabilities
- None — MCP server spec stays unchanged; IDE extensions consume existing MCP protocol

## Approach

Each IDE spawns `forge mcp` on activation via stdio subprocess. The extension/plugin acts as a thin MCP client: sends JSON-RPC requests for tools and prompts, listens for notifications, and renders results in native IDE UI. No duplicate business logic — all capabilities come from the Forge engine through MCP.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `extensions/vscode/` | New | VS Code extension source |
| `extensions/neovim/` | New | Neovim plugin source |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| MCP protocol version mismatch | Low | Negotiate protocol version during initialize handshake |
| IDE-specific API limitations | Medium | Prototype key features early; fallback to simpler UI patterns |
| stdio subprocess lifecycle issues | Low | Handle cleanup on deactivate/detach with kill + timeout |

## Rollback Plan

- VS Code: Uninstall extension or disable via Extensions panel
- Neovim: Remove plugin from lazy.nvim spec and restart Neovim

## Dependencies

- `forge mcp` binary (Phase 11) — must be built and in PATH
- VS Code: `vsce` for packaging, `@types/vscode` for API types
- Neovim: lazy.nvim (recommended), `plenary.nvim` for helpers

## Success Criteria

- [ ] `forge mcp` spawns and initializes in both IDEs on activation
- [ ] Each command (`forge:status`, `forge:diagnose`, `forge:explain`, `forge:run`) produces correct output in both IDEs
- [ ] Diagnostic notifications render as IDE-native markers (problems panel / diagnostics)
- [ ] Extension/plugin deactivates cleanly, killing the subprocess
- [ ] VSIX installs and loads without errors
- [ ] Plugin installs via lazy.nvim without errors
