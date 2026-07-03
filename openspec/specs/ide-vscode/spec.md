# IDE — VS Code Extension Specification

## Purpose

VS Code extension that spawns `anvil mcp` as a stdio subprocess and renders MCP responses in native VS Code UI surfaces — status bar, webview panel, commands, and problems panel. Acts as a thin MCP client with zero anvil business logic; all capabilities come from the anvil engine.

## Requirements

### Requirement: Activation and Subprocess Lifecycle

The extension MUST spawn `anvil mcp` as a stdio child process on workspace activation (when `anvil.toml` is present or a anvil command is invoked). The extension MUST kill the subprocess on deactivation.

#### Scenario: Activates and spawns anvil mcp

- GIVEN the user opens a workspace containing `anvil.toml`
- WHEN the extension activates
- THEN it spawns `anvil mcp` as a child process via stdio transport
- AND the extension enters ready state after receiving the MCP `initialize` response

#### Scenario: Deactivates cleanly

- GIVEN the extension is active with a running `anvil mcp` subprocess
- WHEN the extension deactivates (window close, disable)
- THEN it sends SIGTERM to the subprocess
- AND waits up to 3 seconds before sending SIGKILL if still running

### Requirement: Status Bar Indicator

The extension MUST display a status bar item showing anvil connection status and health score.

#### Scenario: Shows connected state

- GIVEN `anvil mcp` initialized successfully
- WHEN the status bar renders
- THEN it shows "Anvil: {health_score}" with a colored indicator
- AND clicking it runs `Anvil: Show Status`

#### Scenario: Shows disconnected state

- GIVEN `anvil mcp` failed to initialize or terminated
- WHEN the status bar renders
- THEN it shows "Anvil: Disconnected" in red

### Requirement: Commands

The extension MUST register four commands — `Anvil: Show Status`, `Anvil: Diagnose`, `Anvil: Explain Runtime`, `Anvil: Run Command`.

#### Scenario: Show Status displays in webview

- GIVEN anvil mcp is connected
- WHEN the user runs `Anvil: Show Status`
- THEN the extension sends `anvil:status` via MCP
- AND displays the response in a webview panel

#### Scenario: Diagnose populates problems panel

- GIVEN anvil mcp is connected
- WHEN the user runs `Anvil: Diagnose`
- THEN the extension sends `anvil:diagnose` via MCP
- AND renders results in the problems panel

#### Scenario: Explain Runtime prompts for selection

- GIVEN anvil mcp is connected
- WHEN the user runs `Anvil: Explain Runtime`
- THEN the extension prompts for a runtime selection
- AND sends `anvil_explain` with the chosen runtime
- AND displays explanation in a webview

#### Scenario: Run Command prompts for input

- GIVEN anvil mcp is connected
- WHEN the user runs `Anvil: Run Command`
- THEN the extension prompts for command and arguments
- AND sends `anvil_run` with the input
- AND displays output in an output channel

### Requirement: Diagnostics from MCP Notifications

The extension MUST subscribe to `anvil/warning` and `anvil/error` notifications and push them to the VS Code diagnostic collection.

#### Scenario: Warning creates diagnostic marker

- GIVEN anvil mcp is connected and diagnostics are active
- WHEN a `anvil/warning` notification arrives
- THEN the extension creates a Warning diagnostic
- AND adds it to the active diagnostic collection

#### Scenario: Error creates error marker

- GIVEN anvil mcp is connected
- WHEN a `anvil/error` notification arrives
- THEN the extension creates a diagnostic with severity Error
- AND adds it to the active diagnostic collection

### Requirement: Anvil Not Found Handling

The extension MUST handle the `anvil` binary missing from PATH by showing an error with installation instructions.

#### Scenario: Anvil binary not found

- GIVEN the `anvil` binary is not on PATH
- WHEN the extension attempts to spawn `anvil mcp`
- THEN it catches the spawn error
- AND shows a VS Code error notification with install instructions
- AND the status bar shows "Anvil: Not Found"

### Requirement: Platform Requirements

The extension MUST target VS Code 1.82+ and require Node.js 18+ for the extension host.
