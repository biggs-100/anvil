import * as vscode from "vscode";
import { McpClient } from "./mcpClient";
import { AnvilPanel } from "./anvilPanel";
import { AnvilStatusBar } from "./statusBar";

/**
 * Register all four anvil commands with the VS Code extension context.
 *
 * Commands:
 *   - `anvil.status`   → prompts/get anvil:status → webview panel
 *   - `anvil.diagnose` → prompts/get anvil:diagnose → problems panel
 *   - `anvil.explain`  → input box → tools/call anvil_explain → webview panel
 *   - `anvil.run`      → input box → tools/call anvil_run → output channel
 */
export function registerCommands(
  context: vscode.ExtensionContext,
  client: McpClient,
  statusBar: AnvilStatusBar,
  diagnosticCollection: vscode.DiagnosticCollection,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("anvil.status", () => handleStatus(client, statusBar)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("anvil.diagnose", () => handleDiagnose(client, diagnosticCollection)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("anvil.explain", () => handleExplain(client)),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("anvil.run", () => handleRun(client)),
  );
}

/**
 * anvil.status: Show anvil environment status in a webview panel.
 *
 * Sends `prompts/get` with name `anvil:status` and renders the markdown response.
 */
async function handleStatus(client: McpClient, statusBar: AnvilStatusBar): Promise<void> {
  assertConnected(client);

  try {
    const result = await client.getPrompt("anvil:status");
    const text = result.messages?.[0]?.content?.text ?? "No status available.";
    AnvilPanel.show("Anvil: Status", text);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to fetch anvil status: ${msg}`);
  }
}

/**
 * anvil.diagnose: Run anvil diagnostics and populate the problems panel.
 *
 * Sends `prompts/get` with name `anvil:diagnose` and creates diagnostic
 * entries from the findings in the markdown report. Also opens the problems panel.
 */
async function handleDiagnose(
  client: McpClient,
  diagnosticCollection: vscode.DiagnosticCollection,
): Promise<void> {
  assertConnected(client);

  try {
    const result = await client.getPrompt("anvil:diagnose");
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

    diagnosticCollection.set(vscode.Uri.parse("anvil://diagnostics"), diagnostics);
    vscode.commands.executeCommand("workbench.action.problems.focus");
    vscode.window.showInformationMessage(`Anvil diagnose complete — ${diagnostics.length} finding(s)`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to run anvil diagnose: ${msg}`);
  }
}

/**
 * anvil.explain: Show runtime explanation in a webview panel.
 *
 * Prompts the user to select a runtime (via input box), then calls the
 * `anvil_explain` tool and renders the result in the webview.
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
    const result = await client.callTool("anvil_explain", { runtime });
    const text = result.content?.[0]?.text ?? "No explanation available.";

    // Try to pretty-print JSON output
    let displayText = text;
    try {
      const parsed = JSON.parse(text);
      displayText = JSON.stringify(parsed, null, 2);
    } catch {
      // Not JSON — use as-is (likely markdown)
    }

    AnvilPanel.show(`Anvil: Explain ${runtime}`, displayText);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`Failed to explain runtime: ${msg}`);
  }
}

/**
 * anvil.run: Execute a command in the anvil environment.
 *
 * Prompts the user for a command string, then calls `anvil_run` tool and
 * shows the output in an output channel.
 */
async function handleRun(client: McpClient): Promise<void> {
  assertConnected(client);

  const input = await vscode.window.showInputBox({
    prompt: "Enter command to run in anvil environment",
    placeHolder: "node --version",
    ignoreFocusOut: true,
  });

  if (!input) return; // User cancelled

  // Split input into command and args
  const parts = splitCommand(input);
  const cmd = parts[0];
  const args = parts.slice(1);

  const outputChannel = vscode.window.createOutputChannel("Anvil Run");
  outputChannel.show();
  outputChannel.appendLine(`> anvil run ${input}`);
  outputChannel.appendLine("");

  try {
    const result = await client.callTool("anvil_run", { cmd, args });
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
        ? "anvil binary not found. Install anvil and ensure it is on PATH."
        : client.state === "connecting"
          ? "Anvil is still starting up. Please wait a moment and try again."
          : "Anvil is not connected. Open a workspace with anvil.toml to activate.";

    const action = "Install Anvil";
    vscode.window.showErrorMessage(msg, action).then((selected) => {
      if (selected === action) {
        vscode.env.openExternal(vscode.Uri.parse("https://anvil.run/docs/install"));
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
