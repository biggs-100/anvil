import * as vscode from "vscode";
import { McpClient } from "./mcpClient";
import { AnvilStatusBar } from "./statusBar";
import { registerCommands } from "./commands";
import { AnvilPanel } from "./anvilPanel";

/**
 * Activate the Anvil VS Code extension.
 *
 * Responsibilities:
 *   - Spawn `anvil mcp` as a stdio child process via McpClient
 *   - Register 4 commands: anvil.status, anvil.diagnose, anvil.explain, anvil.run
 *   - Create the StatusBarItem showing connection state
   *   - Subscribe to anvil/warning and anvil/error notifications for diagnostics
 *   - Register a TreeDataProvider for the Anvil Explorer view
 *
 * Activation triggers:
 *   - Any of the 4 anvil commands is invoked
 *   - A workspace folder containing `anvil.toml` is opened
 */
export function activate(context: vscode.ExtensionContext): void {
  const client = new McpClient();
  const statusBar = new AnvilStatusBar();
  const diagnosticCollection = vscode.languages.createDiagnosticCollection("anvil");

  context.subscriptions.push(statusBar);
  context.subscriptions.push(diagnosticCollection);
  context.subscriptions.push(AnvilPanel.currentPanel ?? { dispose: () => undefined });
  context.subscriptions.push(client);

  // ── Connect to anvil mcp ───────────────────────────────────────────────
      connectAnvil(client, statusBar);

  // ── Register commands ──────────────────────────────────────────────────
  registerCommands(context, client, statusBar, diagnosticCollection);

  // ── Notification handlers ──────────────────────────────────────────────
  client.onNotification("anvil/warning", (params: unknown) => {
    const p = params as { finding?: string; severity?: string };
    if (p?.finding) {
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 0),
        `[Anvil Warning] ${p.finding}`,
        vscode.DiagnosticSeverity.Warning,
      );
      diagnosticCollection.set(vscode.Uri.parse("anvil://notifications"), [diagnostic]);
    }
  });

  client.onNotification("anvil/error", (params: unknown) => {
    const p = params as { operation?: string; error?: string };
    if (p?.error) {
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(0, 0, 0, 0),
        `[Anvil Error] ${p.operation ?? "unknown"}: ${p.error}`,
        vscode.DiagnosticSeverity.Error,
      );
      diagnosticCollection.set(vscode.Uri.parse("anvil://notifications"), [diagnostic]);
    }
  });

  // ── State change handler ───────────────────────────────────────────────
  client.onNotification("anvil/state_changed", (params: unknown) => {
    const p = params as { new_state?: string };
    if (p?.new_state === "Ready") {
      statusBar.setState("connected", "OK");
    } else if (p?.new_state) {
      statusBar.setState("disconnected");
    }
  });

  // ── Re-connect on state_changed from disconnected to ready ─────────────
  client.onNotification("anvil/state_changed", (params: unknown) => {
    const p = params as { old_state?: string; new_state?: string };
    if (p?.old_state === "disconnected" && p?.new_state && p.new_state !== "Ready") {
      statusBar.setState("connecting");
  connectAnvil(client, statusBar);
    }
  });
}

/**
 * Attempt to connect to anvil mcp.
 * Shows error notification if the binary is not found or connection fails.
 */
async function connectAnvil(client: McpClient, statusBar: AnvilStatusBar): Promise<void> {
  try {
    await client.connect();
    statusBar.setState("connected", "OK");
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes("not found") || msg.includes("ENOENT")) {
      statusBar.setState("not-found");
      const action = "Install Anvil";
      vscode.window.showErrorMessage(
        "anvil binary not found on PATH. Install anvil to use this extension.",
        action,
      ).then((selected) => {
        if (selected === action) {
          vscode.env.openExternal(vscode.Uri.parse("https://anvil.run/docs/install"));
        }
      });
    } else {
      statusBar.setState("disconnected");
      vscode.window.showErrorMessage(`Failed to connect to anvil: ${msg}`);
    }
  }
}

/**
 * Deactivate the Anvil VS Code extension.
 *
 * Kills the `anvil mcp` subprocess and cleans up resources.
 */
export function deactivate(): void {
  // McpClient.close() is called automatically via its disposal in the
  // extension context subscriptions. No additional cleanup needed here.
}
