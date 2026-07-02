import * as vscode from "vscode";
import { McpClient } from "./mcpClient";
import { ForgePanel } from "./forgePanel";
import { ForgeStatusBar } from "./statusBar";

/**
 * Register all four forge commands with the VS Code extension context.
 *
 * Commands:
 *   - `forge.status`   → prompts/get forge:status → webview panel
 *   - `forge.diagnose` → prompts/get forge:diagnose → problems panel
 *   - `forge.explain`  → input box → tools/call forge_explain → webview panel
 *   - `forge.run`      → input box → tools/call forge_run → output channel
 */
export function registerCommands(
  context: vscode.ExtensionContext,
  client: McpClient,
  statusBar: ForgeStatusBar,
  diagnosticCollection: vscode.DiagnosticCollection,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("forge.status", () => handleStatus(client, statusBar)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("forge.diagnose", () => handleDiagnose(client, diagnosticCollection)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("forge.explain", () => handleExplain(client)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("forge.run", () => handleRun(client)),
  );
}

/**
 * forge.status: Show forge environment status in a webview panel.
 *
 * Sends `prompts/get` with name `forge:status` and renders the markdown response.
 */
async function handleStatus(client: McpClient, statusBar: ForgeStatusBar): Promise<void> {
  assertConnected(client);

  try {
    const result = await client.getPrompt("forge:status");
    const text = result.messages?.[0]?.content?.text ?? "No status available.";
    ForgePanel.show("Forge: Status", text);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to fetch forge status: ${msg}`);
  }
}

/**
 * forge.diagnose: Run forge diagnostics and populate the problems panel.
 *
 * Sends `prompts/get` with name `forge:diagnose` and creates diagnostic
 * entries from the findings in the markdown report. Also opens the problems panel.
 */
async function handleDiagnose(
  client: McpClient,
  diagnosticCollection: vscode.DiagnosticCollection,
): Promise<void> {
  assertConnected(client);

  try {
    const result = await client.getPrompt("forge:diagnose");
    const text = result.messages?.[0]?.content?.text ?? "No diagnostic data.";

    // Parse the markdown report for findings and create diagnostics
    const diagnostics: vscode.Diagnostic[] = [];
    const findingRegex = /-\s*\*\*(.+?)\*\*\s*\[(.+?)\]\s*(.+?)(?:\n|$)/g;
    let match: RegExpExecArray | null;

    while ((match = findingRegex.exec(text)) !== null) {
      const [, code, severityStr, message] = match;
      const severity =
        severityStr?.toUpperCase() === "ERROR"
          ? vscode.DiagnosticSeverity.Error
          : severityStr?.toUpperCase() === "CRITICAL"
            ? vscode.DiagnosticSeverity.Error
            : vscode.DiagnosticSeverity.Warning;

      diagnostics.push(
        new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 0),
          `[${code?.trim() ?? "UNKNOWN"}] ${message?.trim() ?? ""}`,
          severity,
        ),
      );
    }

    // If no structured findings were parsed, create a single info diagnostic
    if (diagnostics.length === 0) {
      diagnostics.push(
        new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 0),
          text.length > 500 ? text.substring(0, 500) + "..." : text,
          vscode.DiagnosticSeverity.Information,
        ),
      );
    }

    diagnosticCollection.set(vscode.Uri.parse("forge://diagnostics"), diagnostics);
    vscode.commands.executeCommand("workbench.action.problems.focus");
    vscode.window.showInformationMessage(`Forge diagnose complete — ${diagnostics.length} finding(s)`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to run forge diagnose: ${msg}`);
  }
}

/**
 * forge.explain: Show runtime explanation in a webview panel.
 *
 * Prompts the user to select a runtime (via input box), then calls the
 * `forge_explain` tool and renders the result in the webview.
 */
async function handleExplain(client: McpClient): Promise<void> {
  assertConnected(client);

  const runtime = await vscode.window.showInputBox({
    prompt: "Enter runtime name to explain (e.g., node, python)",
    placeHolder: "node",
    ignoreFocusOut: true,
  });

  if (!runtime) return; // User cancelled

  try {
    const result = await client.callTool("forge_explain", { runtime });
    const text = result.content?.[0]?.text ?? "No explanation available.";

    // Try to pretty-print JSON output
    let displayText = text;
    try {
      const parsed = JSON.parse(text);
      displayText = JSON.stringify(parsed, null, 2);
    } catch {
      // Not JSON — use as-is (likely markdown)
    }

    ForgePanel.show(`Forge: Explain ${runtime}`, displayText);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to explain runtime: ${msg}`);
  }
}

/**
 * forge.run: Execute a command in the forge environment.
 *
 * Prompts the user for a command string, then calls `forge_run` tool and
 * shows the output in an output channel.
 */
async function handleRun(client: McpClient): Promise<void> {
  assertConnected(client);

  const input = await vscode.window.showInputBox({
    prompt: "Enter command to run in forge environment",
    placeHolder: "node --version",
    ignoreFocusOut: true,
  });

  if (!input) return; // User cancelled

  // Split input into command and args
  const parts = splitCommand(input);
  const cmd = parts[0];
  const args = parts.slice(1);

  const outputChannel = vscode.window.createOutputChannel("Forge Run");
  outputChannel.show();
  outputChannel.appendLine(`> forge run ${input}`);
  outputChannel.appendLine("");

  try {
    const result = await client.callTool("forge_run", { cmd, args });
    const text = result.content?.[0]?.text ?? "(no output)";

    // Pretty-print JSON result
    try {
      const parsed = JSON.parse(text);
      outputChannel.appendLine(JSON.stringify(parsed, null, 2));
    } catch {
      outputChannel.appendLine(text);
    }

    if (result.is_error) {
      outputChannel.appendLine("");
      outputChannel.appendLine("[ERROR] Command failed — see above for details.");
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    outputChannel.appendLine(`[ERROR] ${msg}`);
  }
}

/**
 * Assert that the MCP client is in the "ready" state, showing an error if not.
 */
function assertConnected(client: McpClient): void {
  if (client.state !== "ready") {
    const msg =
      client.state === "error"
        ? "forge binary not found. Install forge and ensure it is on PATH."
        : client.state === "connecting"
          ? "Forge is still starting up. Please wait a moment and try again."
          : "Forge is not connected. Open a workspace with forge.toml to activate.";

    const action = "Install Forge";
    vscode.window.showErrorMessage(msg, action).then((selected) => {
      if (selected === action) {
        vscode.env.openExternal(vscode.Uri.parse("https://forge.run/docs/install"));
      }
    });
    throw new Error(msg);
  }
}

/**
 * Split a command string into command and arguments respecting quoted strings.
 *
 * Example: `node --version` → ["node", "--version"]
 *          `echo "hello world"` → ["echo", "hello world"]
 */
function splitCommand(input: string): string[] {
  const parts: string[] = [];
  let current = "";
  let inQuote = false;
  let quoteChar = "";

  for (let i = 0; i < input.length; i++) {
    const ch = input[i];

    if (inQuote) {
      if (ch === quoteChar) {
        inQuote = false;
      } else {
        current += ch;
      }
    } else if (ch === '"' || ch === "'") {
      inQuote = true;
      quoteChar = ch;
    } else if (ch === " ") {
      if (current) {
        parts.push(current);
        current = "";
      }
    } else {
      current += ch;
    }
  }

  if (current) {
    parts.push(current);
  }

  return parts;
}
