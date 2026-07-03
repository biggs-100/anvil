# IDE — Neovim Plugin Specification

## Purpose

Neovim Lua plugin that spawns `anvil mcp` as a stdio subprocess and renders MCP responses in native Neovim UI surfaces — floating windows, quickfix list, Telescope picker, and vim.diagnostics. Acts as a thin MCP client with zero anvil business logic; all capabilities come from the anvil engine.

## Requirements

### Requirement: Activation and Subprocess Lifecycle

The plugin MUST spawn `anvil mcp` as a stdio child process when opening a directory containing `anvil.toml` (via `BufRead` autocmd). The plugin MUST kill the subprocess on `VimLeave`.

#### Scenario: Opens anvil.toml and spawns anvil mcp

- GIVEN the user opens a project directory containing `anvil.toml`
- WHEN Neovim reads the file
- THEN the plugin spawns `anvil mcp` via `vim.fn.jobstart` with stdio transport
- AND the plugin enters ready state after receiving the MCP `initialize` response

#### Scenario: Cleanup on VimLeave

- GIVEN the plugin is active with a running `anvil mcp` subprocess
- WHEN Neovim exits
- THEN the plugin sends `jobstop` to the subprocess
- AND waits up to 3 seconds before force-killing

### Requirement: Commands

The plugin MUST register four user commands — `:AnvilStatus`, `:AnvilDoctor`, `:AnvilExplain`, `:AnvilRun`.

#### Scenario: AnvilStatus shows floating window

- GIVEN anvil mcp is connected
- WHEN the user runs `:AnvilStatus`
- THEN the plugin sends `anvil:status` via MCP
- AND displays the response in a floating window

#### Scenario: AnvilDoctor populates quickfix

- GIVEN anvil mcp is connected
- WHEN the user runs `:AnvilDoctor`
- THEN the plugin sends `anvil:doctor` via MCP
- AND populates the quickfix list with diagnostic results

#### Scenario: AnvilExplain shows runtime details

- GIVEN anvil mcp is connected
- WHEN the user runs `:AnvilExplain {runtime}`
- THEN the plugin sends `anvil_explain` with the given runtime argument
- AND displays the explanation in a floating window

#### Scenario: AnvilRun opens terminal buffer

- GIVEN anvil mcp is connected
- WHEN the user runs `:AnvilRun {cmd} {args}`
- THEN the plugin sends `anvil_run` with the command and arguments
- AND displays output in a new terminal buffer

### Requirement: Telescope Integration

The plugin SHOULD provide a `Telescope anvil` picker listing runtimes and config variables.

#### Scenario: Telescope picker shows anvil resources

- GIVEN anvil mcp is connected
- WHEN the user runs `:Telescope anvil`
- THEN the picker lists available runtimes and config variables
- AND selecting an item shows details in a floating window

### Requirement: Diagnostics from MCP Notifications

The plugin MUST subscribe to `anvil/warning` and `anvil/error` notifications and push them to `vim.diagnostic`.

#### Scenario: Warning creates vim diagnostic

- GIVEN anvil mcp is connected
- WHEN a `anvil/warning` notification arrives
- THEN the plugin creates a diagnostic with `vim.diagnostic.severity.WARN`
- AND adds it to the diagnostic namespace for the current buffer

#### Scenario: Error creates vim diagnostic

- GIVEN anvil mcp is connected
- WHEN a `anvil/error` notification arrives
- THEN the plugin creates a diagnostic with `vim.diagnostic.severity.ERROR`
- AND adds it to the diagnostic namespace for the current buffer

### Requirement: Anvil Not Found Handling

The plugin MUST handle the `anvil` binary missing from PATH by showing an error message with installation instructions.

#### Scenario: Anvil binary not found on open

- GIVEN the `anvil` binary is not on PATH
- WHEN the plugin attempts to spawn `anvil mcp` on `BufRead`
- THEN it catches the jobstart error
- AND shows a `vim.notify` error with install instructions
- AND does not set the plugin as active

### Requirement: Platform Requirements

The plugin MUST require Neovim 0.9+ and the `anvil` binary on PATH.
