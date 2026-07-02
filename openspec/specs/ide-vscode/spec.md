# IDE — VS Code Extension Specification

## Purpose

VS Code extension that spawns `forge mcp` as a stdio subprocess and renders MCP responses in native VS Code UI surfaces — status bar, webview panel, commands, and problems panel. Acts as a thin MCP client with zero forge business logic; all capabilities come from the forge engine.

## Requirements

### Requirement: Activation and Subprocess Lifecycle

The extension MUST spawn `forge mcp` as a stdio child process on workspace activation (when `forge.toml` is present or a forge command is invoked). The extension MUST kill the subprocess on deactivation.

#### Scenario: Activates and spawns forge mcp

- GIVEN the user opens a workspace containing `forge.toml`
- WHEN the extension activates
- THEN it spawns `forge mcp` as a child process via stdio transport
- AND the extension enters ready state after receiving the MCP `initialize` response

#### Scenario: Deactivates cleanly

- GIVEN the extension is active with a running `forge mcp` subprocess
- WHEN the extension deactivates (window close, disable)
- THEN it sends SIGTERM to the subprocess
- AND waits up to 3 seconds before sending SIGKILL if still running

### Requirement: Status Bar Indicator

The extension MUST display a status bar item showing forge connection status and health score.

#### Scenario: Shows connected state

- GIVEN `forge mcp` initialized successfully
- WHEN the status bar renders
- THEN it shows "Forge: {health_score}" with a colored indicator
- AND clicking it runs `Forge: Show Status`

#### Scenario: Shows disconnected state

- GIVEN `forge mcp` failed to initialize or terminated
- WHEN the status bar renders
- THEN it shows "Forge: Disconnected" in red

### Requirement: Commands

The extension MUST register four commands — `Forge: Show Status`, `Forge: Diagnose`, `Forge: Explain Runtime`, `Forge: Run Command`.

#### Scenario: Show Status displays in webview

- GIVEN forge mcp is connected
- WHEN the user runs `Forge: Show Status`
- THEN the extension sends `forge:status` via MCP
- AND displays the response in a webview panel

#### Scenario: Diagnose populates problems panel

- GIVEN forge mcp is connected
- WHEN the user runs `Forge: Diagnose`
- THEN the extension sends `forge:diagnose` via MCP
- AND renders results in the problems panel

#### Scenario: Explain Runtime prompts for selection

- GIVEN forge mcp is connected
- WHEN the user runs `Forge: Explain Runtime`
- THEN the extension prompts for a runtime selection
- AND sends `forge_explain` with the chosen runtime
- AND displays explanation in a webview

#### Scenario: Run Command prompts for input

- GIVEN forge mcp is connected
- WHEN the user runs `Forge: Run Command`
- THEN the extension prompts for command and arguments
- AND sends `forge_run` with the input
- AND displays output in an output channel

### Requirement: Diagnostics from MCP Notifications

The extension MUST subscribe to `forge/warning` and `forge/error` notifications and push them to the VS Code diagnostic collection.

#### Scenario: Warning creates diagnostic marker

- GIVEN forge mcp is connected and diagnostics are active
- WHEN a `forge/warning` notification arrives
- THEN the extension creates a Warning diagnostic
- AND adds it to the active diagnostic collection

#### Scenario: Error creates error marker

- GIVEN forge mcp is connected
- WHEN a `forge/error` notification arrives
- THEN the extension creates a diagnostic with severity Error
- AND adds it to the active diagnostic collection

### Requirement: Forge Not Found Handling

The extension MUST handle the `forge` binary missing from PATH by showing an error with installation instructions.

#### Scenario: Forge binary not found

- GIVEN the `forge` binary is not on PATH
- WHEN the extension attempts to spawn `forge mcp`
- THEN it catches the spawn error
- AND shows a VS Code error notification with install instructions
- AND the status bar shows "Forge: Not Found"

### Requirement: Platform Requirements

The extension MUST target VS Code 1.82+ and require Node.js 18+ for the extension host.
