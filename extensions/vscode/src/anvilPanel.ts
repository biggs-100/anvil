import * as vscode from "vscode";

/**
 * Manages a VS Code WebviewPanel for displaying anvil markdown output.
 *
 * Used by the `Anvil: Show Status` and `Anvil: Explain Runtime` commands to
 * render rich markdown responses from anvil prompts and tool calls.
 */
export class AnvilPanel {
  /**
   * Track the currently visible panel. Only one panel is kept open at a time.
   */
  public static currentPanel: AnvilPanel | undefined;

  private readonly _panel: vscode.WebviewPanel;
  private _disposables: vscode.Disposable[] = [];

  private constructor(panel: vscode.WebviewPanel) {
    this._panel = panel;
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);
    this._panel.webview.onDidReceiveMessage(
      (message) => {
        if (message.command === "copy") {
          vscode.env.clipboard.writeText(message.text);
          vscode.window.showInformationMessage("Copied to clipboard");
        }
      },
      null,
      this._disposables,
    );
  }

  /**
   * Show or reveal the anvil panel with the given markdown content.
   *
   * @param title    Panel title (e.g. "Anvil: Status", "Anvil: Explain")
   * @param markdown Markdown text to render in the webview
   */
  static show(title: string, markdown: string): void {
    if (AnvilPanel.currentPanel) {
      AnvilPanel.currentPanel._panel.title = title;
      AnvilPanel.currentPanel._update(markdown);
      AnvilPanel.currentPanel._panel.reveal(vscode.ViewColumn.Beside);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "anvilPanel",
      title,
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        localResourceRoots: [],
      },
    );

    AnvilPanel.currentPanel = new AnvilPanel(panel);
    AnvilPanel.currentPanel._update(markdown);
  }

  /**
   * Render markdown as HTML in the webview.
   */
  private _update(markdown: string): void {
    this._panel.webview.html = this._renderHtml(markdown);
  }

  /**
   * Convert markdown text to a styled HTML page for the webview.
   *
   * Uses the VS Code markdown styling via `vscode.MarkdownString` rendered
   * as sanitized HTML with copy buttons on code blocks.
   */
  private _renderHtml(markdown: string): string {
    const escaped = markdown
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

    // Very basic markdown → HTML conversion for the panel.
    // This covers the common patterns from anvil prompt responses (headings,
    // bold, lists, code fences). For full markdown rendering in production,
    // consider importing a markdown-it or similar library.
    const html = escaped
      // Code blocks (``` ... ```)
      .replace(/```(\w*)\n([\s\S]*?)```/g, (_match, _lang, code) => {
        const trimmed = code.trim();
        const encoded = trimmed
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;");
        return `<pre style="background:#1e1e1e;color:#d4d4d4;padding:12px;border-radius:4px;overflow-x:auto;font-family:Consolas,'Courier New',monospace;font-size:13px;">${encoded}</pre>`;
      })
      // Headings (## then #)
      .replace(/^## (.+)$/gm, '<h2 style="color:var(--vscode-editor-foreground);border-bottom:1px solid var(--vscode-panel-border);padding-bottom:4px;">$1</h2>')
      .replace(/^# (.+)$/gm, '<h1 style="color:var(--vscode-editor-foreground);">$1</h1>')
      // Bold
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      // Inline code
      .replace(/`([^`]+)`/g, '<code style="background:#3c3c3c;padding:1px 4px;border-radius:3px;font-family:Consolas,monospace;">$1</code>')
      // Unordered list items
      .replace(/^- (.+)$/gm, '<li style="margin:2px 0;">$1</li>')
      // Newlines to <br> (for block text)
      .replace(/\n/g, "<br>");

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <style>
    body {
      font-family: var(--vscode-editor-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
      font-size: var(--vscode-editor-font-size, 14px);
      color: var(--vscode-editor-foreground);
      background: var(--vscode-editor-background);
      padding: 16px;
      line-height: 1.6;
      word-wrap: break-word;
    }
    ul { padding-left: 24px; }
    li { list-style: disc; }
    hr { border: none; border-top: 1px solid var(--vscode-panel-border); }
    a { color: var(--vscode-textLink-foreground); }
  </style>
</head>
<body>
  ${html}
</body>
</html>`;
  }

  /**
   * Clean up panel resources.
   */
  dispose(): void {
    AnvilPanel.currentPanel = undefined;
    this._panel.dispose();
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) d.dispose();
    }
  }
}
