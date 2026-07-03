## Verification Report

**Change**: forge-ide-integration
**Version**: 1.0
**Mode**: Standard

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 15 |
| Tasks complete | 15 |
| Tasks incomplete | 0 |

**All 15 tasks are marked [x] — all files exist on disk as specified.**

### Build & Tests Execution

**Build (Rust — cargo build)**: ✅ Passed
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.41s
```

**Build (TypeScript — tsc)**: ❌ Failed — 2 compilation errors
```text
src/commands.ts(204,7): error TS2367: This comparison appears to be unintentional because the types '"connecting" | "disconnected" | "error"' and '"not-found"' have no overlap.

src/extension.ts(29,30): error TS2345: Argument of type 'McpClient' is not assignable to parameter of type '{ dispose(): any; }'.
  Property 'dispose' is missing in type 'McpClient' but required in type '{ dispose(): any; }'.
```

**Tests**: ➖ Not available (no test framework configured; test_command is empty in openspec/config.yaml)

**Coverage**: ➖ Not available (threshold: 0%)

### Spec Compliance Matrix

#### VS Code Extension (spec: openspec/specs/ide-vscode/spec.md)

| Requirement | Scenario | Implementation Evidence | Result |
|---|---|---|---|
| Activation and Subprocess Lifecycle | Activates and spawns anvil mcp | `extension.ts` — `connectForge()` → `client.connect()` spawns `anvil mcp` via `cp.spawn` | ✅ COMPLIANT |
| Activation and Subprocess Lifecycle | Deactivates cleanly | `extension.ts` `deactivate()`, `mcpClient.ts` `close()` — shutdown notification, stdin EOF, 3s SIGKILL grace | ✅ COMPLIANT |
| Status Bar Indicator | Shows connected state | `statusBar.ts` `setState("connected", health)` — shows `$(check) Anvil: OK` | ✅ COMPLIANT |
| Status Bar Indicator | Shows disconnected state | `statusBar.ts` `setState("disconnected")` — shows `$(warning) Anvil: Disconnected` with error background | ✅ COMPLIANT |
| Commands | Show Status displays in webview | `commands.ts` `handleStatus()` → `client.getPrompt("anvil:status")` → `ForgePanel.show()` | ✅ COMPLIANT |
| Commands | Diagnose populates problems panel | `commands.ts` `handleDiagnose()` → `client.getPrompt("anvil:diagnose")` → `diagnosticCollection.set()` | ✅ COMPLIANT |
| Commands | Explain Runtime prompts for selection | `commands.ts` `handleExplain()` → `showInputBox()` → `client.callTool("anvil_explain", {runtime})` → webview | ✅ COMPLIANT |
| Commands | Run Command prompts for input | `commands.ts` `handleRun()` → `showInputBox()` → `client.callTool("anvil_run", {cmd, args})` → output channel | ✅ COMPLIANT |
| Diagnostics from MCP Notifications | Warning creates diagnostic marker | `extension.ts` `onNotification("anvil/warning", ...)` → `vscode.Diagnostic(Warning)` → `diagnosticCollection.set()` | ✅ COMPLIANT |
| Diagnostics from MCP Notifications | Error creates error marker | `extension.ts` `onNotification("anvil/error", ...)` → `vscode.Diagnostic(Error)` → `diagnosticCollection.set()` | ✅ COMPLIANT |
| Anvil Not Found Handling | Anvil binary not found | `extension.ts` `connectForge()` catches ENOENT → `statusBar.setState("not-found")` + error notification with install link | ✅ COMPLIANT |
| Platform Requirements | VS Code 1.82+, Node 18+ | `package.json` engine `^1.82.0`, tsconfig target `ES2020` (Node 18+), `@types/node ^18.0.0` | ✅ COMPLIANT |

#### Neovim Plugin (spec: openspec/specs/ide-neovim/spec.md)

| Requirement | Scenario | Implementation Evidence | Result |
|---|---|---|---|
| Activation and Subprocess Lifecycle | Opens anvil.toml and spawns anvil mcp | `init.lua` `BufRead` autocmd on `anvil.toml` → `mcp.start()` spawns via `vim.fn.jobstart` | ✅ COMPLIANT |
| Activation and Subprocess Lifecycle | Cleanup on VimLeave | `init.lua` `VimLeavePre` autocmd → `mcp.stop()` — shutdown notification, stdin close, 3s defer + `jobstop` | ✅ COMPLIANT |
| Commands | AnvilStatus shows floating window | `ui.lua` `show_status()` → `mcp.get_prompt("anvil:status")` → `_open_float()` | ✅ COMPLIANT |
| Commands | AnvilDoctor populates quickfix | `ui.lua` `show_diagnose()` → `mcp.get_prompt("anvil:diagnose")` → `vim.fn.setqflist()` + `copen` | ✅ COMPLIANT |
| Commands | AnvilExplain shows runtime details | `ui.lua` `show_explain()` → `mcp.call_tool("anvil_explain", {runtime})` → `_open_float()` | ✅ COMPLIANT |
| Commands | AnvilRun opens terminal buffer | `ui.lua` `show_run()` → `mcp.call_tool("anvil_run", {cmd, args})` → scratch buffer in split window | ✅ COMPLIANT |
| Telescope Integration | Telescope picker shows anvil resources | `ui.lua` `telescope_picker()` defined; `init.lua` calls `load_extension("anvil")` | ⚠️ PARTIAL — picker function exists but no Telescope extension file found (`extensions/neovim/lua/telescope/_extensions/anvil.lua` missing) |
| Diagnostics from MCP Notifications | Warning creates vim diagnostic | `mcp.lua` `_handle_notification("anvil/warning")` → `vim.diagnostic.add(WARN)` | ✅ COMPLIANT |
| Diagnostics from MCP Notifications | Error creates vim diagnostic | `mcp.lua` `_handle_notification("anvil/error")` → `vim.notify(ERROR)` | ⚠️ PARTIAL — uses `vim.notify` instead of `vim.diagnostic.add(ERROR)` which spec requires |
| Anvil Not Found Handling | Anvil binary not found on open | `mcp.lua` `start()` checks `vim.fn.executable("anvil")` → `vim.notify(ERROR)` with install instructions | ✅ COMPLIANT |
| Platform Requirements | Neovim 0.9+, anvil on PATH | `health.lua` checks `MIN_NVIM_VERSION = {0, 9, 0}` and `executable("anvil")` | ✅ COMPLIANT |

**Compliance summary**: 19/21 scenarios compliant, 2 partially compliant, 0 untested

### Correctness (Static Evidence)

| Requirement | Status | Notes |
|---|---|---|
| VS Code: Activation and Subprocess Lifecycle | ✅ Implemented | `extension.ts` activate/deactivate + `mcpClient.ts` spawn/close |
| VS Code: Status Bar Indicator | ✅ Implemented | `statusBar.ts` — 4 states (connected, disconnected, not-found, connecting) |
| VS Code: Commands (4) | ✅ Implemented | `commands.ts` — status, diagnose, explain, run with proper MCP mappings |
| VS Code: Diagnostics from Notifications | ✅ Implemented | `extension.ts` — forge/warning → diagnostic, forge/error → diagnostic |
| VS Code: Anvil Not Found | ✅ Implemented | ENOENT catch → error notification + install link + status bar state |
| VS Code: Platform (1.82+, Node 18+) | ✅ Implemented | Manifest and config match spec |
| Neovim: Activation and Subprocess Lifecycle | ✅ Implemented | `init.lua` autocmds + `mcp.lua` jobstart/jobstop |
| Neovim: Commands (4) | ✅ Implemented | `init.lua` user commands + `ui.lua` renderers |
| Neovim: Telescope Integration | ⚠️ Partial | UI code exists but Telescope extension loader file is missing |
| Neovim: Diagnostics from Notifications | ⚠️ Partial | Warning uses vim.diagnostic (correct); error uses vim.notify (should be vim.diagnostic per spec) |
| Neovim: Anvil Not Found | ✅ Implemented | `mcp.lua` executable check + vim.notify error |
| Neovim: Platform (0.9+, anvil on PATH) | ✅ Implemented | health.lua verification |
| Neovim: Health check | ❌ Bug | Module table `local M = {}` missing — `function M.check()` will error at runtime |
| VS Code: TypeScript compilation | ❌ Failed | 2 errors: McpClient missing dispose(), unreachable "not-found" check |

### Coherence (Design)

| Decision | Followed? | Notes |
|---|---|---|
| Independent MCP client impls (no shared lib) | ✅ Yes | TS `McpClient` in `mcpClient.ts`, Lua `mcp` module in `mcp.lua` — both ~380-420 lines |
| VS Code: WebviewPanel for status/explain | ✅ Yes | `forgePanel.ts` — WebviewPanel with markdown → HTML rendering |
| Neovim: Both float + Telescope | ⚠️ Partial | Float windows for core commands (✅), Telescope picker function exists but extension file missing (❌) |
| Single job for anvil mcp | ✅ Yes | Both IDEs use single process model with cached reference |
| Pending requests map for response routing | ✅ Yes | `Map<id, {resolve,reject}>` in TS, `pending[id]` table in Lua |
| MCP Method Mapping (spec→design→impl) | ✅ Yes | All 4 commands map correctly: `forge:status`→prompts/get, `forge:diagnose`→prompts/get, `forge_explain`→tools/call, `forge_run`→tools/call |
| Notification Contract shapes | ✅ Yes | `forge/warning {finding, severity}` and `forge/error {operation, error}` handled in both IDEs |
| Shutdown sequence: notification → close stdin → 3s wait → kill | ✅ Yes | Both IDEs implement this pattern faithfully |
| File structure per `design.md` File Changes table | ✅ Yes | All 14 files created as specified (minus tsconfig.json, .vscodeignore, health.lua which are also present) |

### Issues Found

**CRITICAL**:
1. ❌ **TypeScript compilation fails** — `openspec/changes/forge-ide-integration/`
   - `mcpClient.ts: McpClient` has no `dispose()` method, so `extension.ts:29` (`context.subscriptions.push(client)`) fails type-checking. Fix: add `dispose(): void { this.close(); }` to `McpClient`.
   - `commands.ts:204` checks `client.state === "not-found"` but `McpClientState` type is `"connecting" | "ready" | "disconnected" | "error"` with no `"not-found"`. Fix: remove the `"not-found"` branch (this state exists only on the StatusBar, not the MCP client).

2. ❌ **health.lua module table not declared** — `extensions/neovim/lua/forge/health.lua` is missing `local M = {}` before `function M.check()` (line 17). Line 14 has `local mcp = nil` instead. The `:checkhealth forge` command will fail at runtime because `M` is `nil` when `M.check` is defined. Fix: replace `local mcp = nil` with `local M = {}`.

3. ❌ **Telescope extension file missing** — `extensions/neovim/lua/forge/init.lua` calls `require("telescope").load_extension("anvil")` but no `extensions/neovim/lua/telescope/_extensions/anvil.lua` exists. The Telescope integration silently fails. Fix: create the extension entry point that delegates to `require("anvil.ui").telescope_picker()`.

**WARNING**:
1. ⚠️ **Neovim error notification uses vim.notify instead of vim.diagnostic** — `extensions/neovim/lua/forge/mcp.lua:410-413` handles `forge/error` with `vim.notify`, but the spec (ide-neovim Requirement: Diagnostics from MCP Notifications) says error should create a diagnostic with `vim.diagnostic.severity.ERROR`, consistent with the warning handler. Warning correctly uses `vim.diagnostic.add(WARN)` at line 399.

2. ⚠️ **Duplicate notification handlers for forge/state_changed** — `extension.ts:63-79` registers two separate `onNotification("anvil/state_changed", ...)` handlers. While functionally correct, they could be consolidated into one handler to reduce confusion about overlapping reconnection logic.

**SUGGESTION**:
1. 💡 **ForgePanel.currentPanel in context.subscriptions** — `extension.ts:28` pushes `ForgePanel.currentPanel ?? { dispose: () => undefined }` to subscriptions, but this is a snapshot. If a panel is created later (via `ForgePanel.show()`), the new panel won't be in subscriptions and won't auto-dispose on deactivation. Consider tracking all created panels.

2. 💡 **SplitCommand utility location** — `commands.ts` has a custom `splitCommand()` function (lines 226-258). The Lua `ui.lua` has its own simpler whitespace split (`:gmatch("%S+")`). Consider extracting shared parsing logic or at least documenting the difference.

### Verdict

**FAIL**

The implementation has 3 CRITICAL issues: TypeScript compilation errors (TypeScript code cannot be built), a missing module table in health.lua that breaks `:checkhealth forge` at runtime, and a missing Telescope extension file that causes a silent feature failure. These must be fixed before the change can be considered verified.
