import * as vscode from "vscode";

/**
 * Status bar states reflecting the anvil MCP connection.
 */
export type StatusBarState = "connected" | "disconnected" | "not-found" | "connecting";

/**
 * Manages the VS Code StatusBarItem for anvil connection status.
 *
 * Three visual states:
 * - `connected`:   "Anvil: {health}" with a green-ish background / foreground
 * - `disconnected`:"Anvil: Disconnected" in red
 * - `not-found`:   "Anvil: Not Found" in red (binary missing from PATH)
 * - `connecting`:  "Anvil: Starting..." in yellow
 *
 * Clicking the status bar item executes the `Anvil: Show Status` command.
 */
export class AnvilStatusBar {
  private _item: vscode.StatusBarItem;

  constructor() {
    this._item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this._item.command = "anvil.status";
    this._item.tooltip = "Click to show anvil status";
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
        this._item.text = `$(check) Anvil: ${health ?? "OK"}`;
        this._item.backgroundColor = undefined;
        this._item.color = undefined;
        break;
      }
      case "disconnected": {
        this._item.text = "$(warning) Anvil: Disconnected";
        this._item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
        this._item.color = undefined;
        break;
      }
      case "not-found": {
        this._item.text = "$(error) Anvil: Not Found";
        this._item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
        this._item.color = undefined;
        this._item.tooltip = "anvil binary not found on PATH. Install anvil and try again.";
        break;
      }
      case "connecting": {
        this._item.text = "$(sync~spin) Anvil: Starting...";
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
