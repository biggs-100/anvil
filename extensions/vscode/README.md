# Anvil VS Code Extension

Thin MCP client that connects VS Code to the [Anvil](https://anvil.run) engine via `anvil mcp`. Displays environment status, diagnostics, runtime explanations, and command output in native VS Code UI surfaces.

## Requirements

- **VS Code** 1.82+
- **Node.js** 18+ (extension host)
- **anvil** binary on PATH ([install guide](https://anvil.run/docs/install))

## Features

### Commands

| Command | Description | MCP Method |
|---------|-------------|------------|
| `Anvil: Show Status` | Display environment state, runtimes, and health | `prompts/get anvil:status` |
| `Anvil: Diagnose` | Run health diagnostics and populate the Problems panel | `prompts/get anvil:diagnose` |
| `Anvil: Explain Runtime` | Show detailed configuration for a specific runtime | `tools/call anvil_explain` |
| `Anvil: Run Command` | Execute a command in the anvil environment | `tools/call anvil_run` |

### Status Bar

A status bar item shows the current connection state:

- **Anvil: OK** — connected and healthy
- **Anvil: Starting...** — connecting to `anvil mcp`
- **Anvil: Disconnected** — connection lost or not started
- **Anvil: Not Found** — anvil binary missing from PATH

Click the status bar item to run `Anvil: Show Status`.

### Diagnostics

The extension subscribes to `anvil/warning` and `anvil/error` notifications and displays them as VS Code diagnostics in the Problems panel.

### Anvil Explorer

The activity bar includes a Anvil panel with a tree view of runtimes and configuration.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `anvil.binaryPath` | `"anvil"` | Path or name of the anvil binary |
| `anvil.healthPollInterval` | `30000` | Health status polling interval in ms (0 to disable) |

## Activation

The extension activates automatically when you open a workspace containing `anvil.toml`, or when you run any anvil command.

## Development

```bash
cd extensions/vscode
npm install
npm run compile
```

Press F5 in VS Code to launch a new Extension Development Host window.

## Extension Structure

```
extensions/vscode/
├── src/
│   ├── extension.ts      # Activation entry point
│   ├── mcpClient.ts      # MCP stdio client (spawn, request, response routing)
│   ├── types.ts          # TypeScript interfaces for MCP types
│   ├── commands.ts       # Command handlers (status, diagnose, explain, run)
│   ├── statusBar.ts      # Status bar indicator
│   └── anvilPanel.ts     # Webview panel for markdown rendering
├── package.json          # Extension manifest
├── tsconfig.json         # TypeScript configuration
├── .vscodeignore         # VSIX packaging filter
└── README.md             # This file
```
