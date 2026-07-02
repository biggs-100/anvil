# Forge VS Code Extension

Thin MCP client that connects VS Code to the [Forge](https://forge.run) engine via `forge mcp`. Displays environment status, diagnostics, runtime explanations, and command output in native VS Code UI surfaces.

## Requirements

- **VS Code** 1.82+
- **Node.js** 18+ (extension host)
- **forge** binary on PATH ([install guide](https://forge.run/docs/install))

## Features

### Commands

| Command | Description | MCP Method |
|---------|-------------|------------|
| `Forge: Show Status` | Display environment state, runtimes, and health | `prompts/get forge:status` |
| `Forge: Diagnose` | Run health diagnostics and populate the Problems panel | `prompts/get forge:diagnose` |
| `Forge: Explain Runtime` | Show detailed configuration for a specific runtime | `tools/call forge_explain` |
| `Forge: Run Command` | Execute a command in the forge environment | `tools/call forge_run` |

### Status Bar

A status bar item shows the current connection state:

- **Forge: OK** — connected and healthy
- **Forge: Starting...** — connecting to `forge mcp`
- **Forge: Disconnected** — connection lost or not started
- **Forge: Not Found** — forge binary missing from PATH

Click the status bar item to run `Forge: Show Status`.

### Diagnostics

The extension subscribes to `forge/warning` and `forge/error` notifications and displays them as VS Code diagnostics in the Problems panel.

### Forge Explorer

The activity bar includes a Forge panel with a tree view of runtimes and configuration.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `forge.binaryPath` | `"forge"` | Path or name of the forge binary |
| `forge.healthPollInterval` | `30000` | Health status polling interval in ms (0 to disable) |

## Activation

The extension activates automatically when you open a workspace containing `forge.toml`, or when you run any forge command.

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
│   └── forgePanel.ts     # Webview panel for markdown rendering
├── package.json          # Extension manifest
├── tsconfig.json         # TypeScript configuration
├── .vscodeignore         # VSIX packaging filter
└── README.md             # This file
```
