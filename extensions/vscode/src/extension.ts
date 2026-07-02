import * as vscode from "vscode";
import { McpClient } from "./mcpClient";
import { ForgeStatusBar } from "./statusBar";
import { registerCommands } from "./commands";
import { ForgePanel } from "./forgePanel";

/**
 * Activate the Forge VS Code extension.
 *
 * Responsibilities:
 *   - Spawn `forge mcp` as a stdio child process via McpClient
 *   - Register 4 commands: forge.status, forge.diagnose, forge.explain, forge.run
 *   - Create the StatusBarItem showing connection state
 *   - Subscribe to forge/warning and forge/error notifications for diagnostics
 *   - Register a TreeDataProvider for the Forge Explorer view
 *
 * Activation triggers:
 *   - Any of the 4 forge commands is invoked
 *   - A workspace folder containing `forge.toml` is opened
 */
export function activate(context: vscode.ExtensionContext): void {
  const client = new McpClient();
  const statusBar = new ForgeStatusBar();
  const diagnosticCollection = vscode.languages.createDiagnosticCollection("forge");

  context.subscriptions.push(statusBar);
  context.subscriptions.push(diagnosticCollection);
  context.subscriptions.push(ForgePanel.currentPanel ?? { dispose: () => undefined });
  context.subscriptions.push(client);

  // ── Connect to forge mcp ───────────────────────────────────────────────
  connectForge(client, statusBar);

  // ── Register commands ──────────────────────────────────────────────────
  registerCommands(context, client, statusBar, diagnosticCollection);

  // ── Notification handlers ──────────────────────────────────────────────
  client.onNotification("forge/warning", (params: unknown) => {
    const p = params as { finding?: string; severity?: string };
    if (p?.finding) {
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 0),
        `[Forge Warning] ${p.finding}`,
        vscode.DiagnosticSeverity.Warning,
      );
      diagnosticCollection.set(vscode.Uri.parse("forge://notifications"), [diagnostic]);
    }
  });

  client.onNotification("forge/error", (params: unknown) => {
    const p = params as { operation?: string; error?: string };
    if (p?.error) {
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 0),
        `[Forge Error] ${p.operation ?? "unknown"}: ${p.error}`,
        vscode.DiagnosticSeverity.Error,
      );
      diagnosticCollection.set(vscode.Uri.parse("forge://notifications"), [diagnostic]);
    }
  });

  // ── State change handler ───────────────────────────────────────────────
  client.onNotification("forge/state_changed", (params: unknown) => {
    const p = params as { new_state?: string };
    if (p?.new_state === "Ready") {
      statusBar.setState("connected", "OK");
    } else if (p?.new_state) {
      statusBar.setState("disconnected");
    }
  });

  // ── Re-connect on state_changed from disconnected to ready ─────────────
  client.onNotification("forge/state_changed", (params: unknown) => {
    const p = params as { old_state?: string; new_state?: string };
    if (p?.old_state === "disconnected" && p?.new_state && p.new_state !== "Ready") {
      statusBar.setState("connecting");
      connectForge(client, statusBar);
    }
  });
}

/**
 * Attempt to connect to forge mcp.
 * Shows error notification if the binary is not found or connection fails.
 */
async function connectForge(client: McpClient, statusBar: ForgeStatusBar): Promise<void> {
  try {
    await client.connect();
    statusBar.setState("connected", "OK");
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes("not found") || msg.includes("ENOENT")) {
      statusBar.setState("not-found");
      const action = "Install Forge";
      vscode.window.showErrorMessage(
        "forge binary not found on PATH. Install forge to use this extension.",
        action,
      ).then((selected) => {
        if (selected === action) {
          vscode.env.openExternal(vscode.Uri.parse("https://forge.run/docs/install"));
        }
      });
    } else {
      statusBar.setState("disconnected");
      vscode.window.showErrorMessage(`Failed to connect to forge: ${msg}`);
    }
  }
}

/**
 * Deactivate the Forge VS Code extension.
 *
 * Kills the `forge mcp` subprocess and cleans up resources.
 */
export function deactivate(): void {
  // McpClient.close() is called automatically via its disposal in the
  // extension context subscriptions. No additional cleanup needed here.
}
