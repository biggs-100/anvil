import * as vscode from "vscode";

/**
 * Status bar states reflecting the forge MCP connection.
 */
export type StatusBarState = "connected" | "disconnected" | "not-found" | "connecting";

/**
 * Manages the VS Code StatusBarItem for forge connection status.
 *
 * Three visual states:
 * - `connected`:   "Forge: {health}" with a green-ish background / foreground
 * - `disconnected`:"Forge: Disconnected" in red
 * - `not-found`:   "Forge: Not Found" in red (binary missing from PATH)
 * - `connecting`:  "Forge: Starting..." in yellow
 *
 * Clicking the status bar item executes the `Forge: Show Status` command.
 */
export class ForgeStatusBar {
  private _item: vscode.StatusBarItem;

  constructor() {
    this._item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this._item.command = "forge.status";
    this._item.tooltip = "Click to show forge status";
    this.setState("connecting");
    this._item.show();
  }

  /**
   * Update the status bar to reflect a new connection state.
   *
   * @param state  One of the four StatusBarState values
   * @param health Optional health score string shown when state is `connected`
   */
  setState(state: StatusBarState, health?: string): void {
    switch (state) {
      case "connected": {
        this._item.text = `$(check) Forge: ${health ?? "OK"}`;
        this._item.backgroundColor = undefined;
        this._item.color = undefined;
        break;
      }
      case "disconnected": {
        this._item.text = "$(warning) Forge: Disconnected";
        this._item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
        this._item.color = undefined;
        break;
      }
      case "not-found": {
        this._item.text = "$(error) Forge: Not Found";
        this._item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
        this._item.color = undefined;
        this._item.tooltip = "forge binary not found on PATH. Install forge and try again.";
        break;
      }
      case "connecting": {
        this._item.text = "$(sync~spin) Forge: Starting...";
        this._item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
        this._item.color = undefined;
        break;
      }
    }
  }

  /**
   * Release the underlying StatusBarItem.
   */
  dispose(): void {
    this._item.dispose();
  }
}
