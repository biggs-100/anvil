# IDE — Neovim Plugin Specification

## Purpose

Neovim Lua plugin that spawns `forge mcp` as a stdio subprocess and renders MCP responses in native Neovim UI surfaces — floating windows, quickfix list, Telescope picker, and vim.diagnostics. Acts as a thin MCP client with zero forge business logic; all capabilities come from the forge engine.

## Requirements

### Requirement: Activation and Subprocess Lifecycle

The plugin MUST spawn `forge mcp` as a stdio child process when opening a directory containing `forge.toml` (via `BufRead` autocmd). The plugin MUST kill the subprocess on `VimLeave`.

#### Scenario: Opens forge.toml and spawns forge mcp

- GIVEN the user opens a project directory containing `forge.toml`
- WHEN Neovim reads the file
- THEN the plugin spawns `forge mcp` via `vim.fn.jobstart` with stdio transport
- AND the plugin enters ready state after receiving the MCP `initialize` response

#### Scenario: Cleanup on VimLeave

- GIVEN the plugin is active with a running `forge mcp` subprocess
- WHEN Neovim exits
- THEN the plugin sends `jobstop` to the subprocess
- AND waits up to 3 seconds before force-killing

### Requirement: Commands

The plugin MUST register four user commands — `:ForgeStatus`, `:ForgeDoctor`, `:ForgeExplain`, `:ForgeRun`.

#### Scenario: ForgeStatus shows floating window

- GIVEN forge mcp is connected
- WHEN the user runs `:ForgeStatus`
- THEN the plugin sends `forge:status` via MCP
- AND displays the response in a floating window

#### Scenario: ForgeDoctor populates quickfix

- GIVEN forge mcp is connected
- WHEN the user runs `:ForgeDoctor`
- THEN the plugin sends `forge:doctor` via MCP
- AND populates the quickfix list with diagnostic results

#### Scenario: ForgeExplain shows runtime details

- GIVEN forge mcp is connected
- WHEN the user runs `:ForgeExplain {runtime}`
- THEN the plugin sends `forge_explain` with the given runtime argument
- AND displays the explanation in a floating window

#### Scenario: ForgeRun opens terminal buffer

- GIVEN forge mcp is connected
- WHEN the user runs `:ForgeRun {cmd} {args}`
- THEN the plugin sends `forge_run` with the command and arguments
- AND displays output in a new terminal buffer

### Requirement: Telescope Integration

The plugin SHOULD provide a `Telescope forge` picker listing runtimes and config variables.

#### Scenario: Telescope picker shows forge resources

- GIVEN forge mcp is connected
- WHEN the user runs `:Telescope forge`
- THEN the picker lists available runtimes and config variables
- AND selecting an item shows details in a floating window

### Requirement: Diagnostics from MCP Notifications

The plugin MUST subscribe to `forge/warning` and `forge/error` notifications and push them to `vim.diagnostic`.

#### Scenario: Warning creates vim diagnostic

- GIVEN forge mcp is connected
- WHEN a `forge/warning` notification arrives
- THEN the plugin creates a diagnostic with `vim.diagnostic.severity.WARN`
- AND adds it to the diagnostic namespace for the current buffer

#### Scenario: Error creates vim diagnostic

- GIVEN forge mcp is connected
- WHEN a `forge/error` notification arrives
- THEN the plugin creates a diagnostic with `vim.diagnostic.severity.ERROR`
- AND adds it to the diagnostic namespace for the current buffer

### Requirement: Forge Not Found Handling

The plugin MUST handle the `forge` binary missing from PATH by showing an error message with installation instructions.

#### Scenario: Forge binary not found on open

- GIVEN the `forge` binary is not on PATH
- WHEN the plugin attempts to spawn `forge mcp` on `BufRead`
- THEN it catches the jobstart error
- AND shows a `vim.notify` error with install instructions
- AND does not set the plugin as active

### Requirement: Platform Requirements

The plugin MUST require Neovim 0.9+ and the `forge` binary on PATH.
