import * as cp from "child_process";
import { EventEmitter } from "events";
import * as readline from "readline";
import {
  McpResponse,
  McpNotification,
  CallToolResult,
  GetPromptResult,
  ListToolsResult,
  McpClientState,
} from "./types";

/**
 * MCP client that spawns `anvil mcp` as a stdio subprocess, sends JSON-RPC 2.0
 * requests, and receives line-delimited responses. Uses a pending-request map
 * to route responses back to callers and emits events for server-sent notifications.
 *
 * Lifecycle:
 *   1. `connect()` – spawns the process, performs the MCP initialize handshake
 *   2. `callTool()` / `getPrompt()` / `listTools()` – standard MCP interactions
 *   3. `close()` – sends shutdown, kills process with 3-second grace period
 */
export class McpClient {
  private _process: cp.ChildProcess | null = null;
  private _rl: readline.Interface | null = null;
  private _state: McpClientState = "disconnected";
  private _nextId = 1;
  private _pending = new Map<
    number | string,
    { resolve: (value: McpResponse) => void; reject: (reason: Error) => void; timer: NodeJS.Timeout }
  >();
  private _emitter = new EventEmitter();
  // Cap the number of pending requests to prevent resource leaks
  private static readonly MAX_PENDING = 128;
  // Request timeout in milliseconds (30 seconds)
  private static readonly REQUEST_TIMEOUT_MS = 30_000;

  get state(): McpClientState {
    return this._state;
  }

  /**
   * Register a handler for server-sent notifications (e.g. anvil/warning, anvil/error).
   */
  onNotification(method: string, handler: (params: unknown) => void): void {
    this._emitter.on(method, handler);
  }

  /**
   * Remove a notification handler previously registered via `onNotification`.
   */
  offNotification(method: string, handler: (params: unknown) => void): void {
    this._emitter.off(method, handler);
  }

  /**
   * Spawn `anvil mcp` and perform the MCP initialize handshake.
   *
   * Rejects if the binary is not found, the process fails to start, or the
   * initialize handshake does not complete within the timeout.
   */
  connect(): Promise<void> {
    if (this._state === "connecting" || this._state === "ready") {
      return Promise.reject(new Error("MCP client is already connected or connecting"));
    }

    this._setState("connecting");

    return new Promise<void>((resolve, reject) => {
      try {
        const proc = cp.spawn("anvil", ["mcp"], {
          stdio: ["pipe", "pipe", "pipe"],
          windowsHide: true,
        });
        this._process = proc;

        let stderrBuf = "";

        // Collect stderr for error reporting
        if (proc.stderr) {
          proc.stderr.on("data", (chunk: Buffer) => {
            stderrBuf += chunk.toString();
          });
        }

        // Set up line-buffered reading from stdout
        const rl = readline.createInterface({ input: proc.stdout!, crlfDelay: Infinity });
        this._rl = rl;

        rl.on("line", (line: string) => {
          const trimmed = line.trim();
          if (!trimmed) return;
          this._handleMessage(trimmed);
        });

        // Handle process exit unexpectedly
        const onExit = (code: number | null, signal: string | null) => {
          this._setState("disconnected");
          this._cleanup();

          if (code !== 0 && stderrBuf && this._pending.size > 0) {
            const errMsg = stderrBuf.includes("not found") || stderrBuf.includes("No such file")
              ? "anvil binary not found on PATH. Install anvil and try again."
              : `anvil mcp exited unexpectedly (code=${code}, signal=${signal}): ${stderrBuf.trim()}`;
            // Reject all pending requests
            for (const [, entry] of this._pending) {
              clearTimeout(entry.timer);
              entry.reject(new Error(errMsg));
            }
            this._pending.clear();
          }
        };

        proc.on("exit", onExit);
        proc.on("error", (err: Error) => {
          this._setState("error");
          this._cleanup();

          const msg = (err as NodeJS.ErrnoException).code === "ENOENT"
            ? "anvil binary not found on PATH. Install anvil and try again."
            : `Failed to spawn anvil mcp: ${err.message}`;
          reject(new Error(msg));
        });

        // Wait a tick for the process to start, then send initialize
        setImmediate(() => {
          if (!proc.pid && !proc.killed) {
            // The error handler should have fired, but as a safety net:
            reject(new Error("anvil mcp process failed to start"));
            return;
          }

          // Send initialize request
          this._sendRawRequest("initialize", {
            protocol_version: "2024-11-05",
            capabilities: {},
            client_info: { name: "anvil-vscode", version: "0.1.0" },
          })
            .then((_resp) => {
              // Send initialized notification (no response expected)
              this._sendNotification("notifications/initialized", {});
              this._setState("ready");
              resolve();
            })
            .catch((err: Error) => {
              this._setState("error");
              reject(err);
            });
        });
      } catch (err) {
        const msg = err instanceof Error && (err as NodeJS.ErrnoException).code === "ENOENT"
          ? "anvil binary not found on PATH. Install anvil and try again."
          : `Failed to connect to anvil mcp: ${err instanceof Error ? err.message : String(err)}`;
        this._setState("error");
        reject(new Error(msg));
      }
    });
  }

  /**
   * Send a JSON-RPC 2.0 request and wait for the matching response.
   */
  sendRequest(method: string, params?: unknown): Promise<McpResponse> {
    return this._sendRawRequest(method, params);
  }

  /**
   * Call an MCP tool and return the parsed result.
   */
  async callTool(name: string, args?: Record<string, unknown>): Promise<CallToolResult> {
    const response = await this._sendRawRequest("tools/call", { name, arguments: args ?? {} });
    if (response.error) {
      throw new Error(`Tool call failed: ${response.error.message}`);
    }
    return response.result as CallToolResult;
  }

  /**
   * Get a prompt result from the MCP server.
   */
  async getPrompt(name: string, args?: Record<string, unknown>): Promise<GetPromptResult> {
    const response = await this._sendRawRequest("prompts/get", { name, arguments: args ?? {} });
    if (response.error) {
      throw new Error(`Prompt failed: ${response.error.message}`);
    }
    return response.result as GetPromptResult;
  }

  /**
   * List available tools from the MCP server.
   */
  async listTools(): Promise<ListToolsResult> {
    const response = await this._sendRawRequest("tools/list", {});
    if (response.error) {
      throw new Error(`List tools failed: ${response.error.message}`);
    }
    return response.result as ListToolsResult;
  }

  /**
   * Gracefully shut down the MCP connection. Sends shutdown notification,
   * closes stdin (signals EOF to anvil mcp), then waits up to 3 seconds
   * before force-killing the process.
   */
  close(): void {
    if (!this._process || this._state === "disconnected") {
      return;
    }

    this._setState("disconnected");

    // Send shutdown notification
    this._sendNotification("shutdown", {});

    // Close stdin to signal EOF to anvil mcp
    if (this._process.stdin) {
      this._process.stdin.end();
    }

    // Cleanup event listeners on the readline interface
    this._cleanup();

    // Grace period: wait 3s then SIGKILL if still alive
    const proc = this._process;
    const killTimer = setTimeout(() => {
      if (proc && !proc.killed) {
        proc.kill("SIGKILL");
      }
    }, 3000);

    proc.once("exit", () => {
      clearTimeout(killTimer);
    });

    // Also try SIGTERM immediately
    if (proc.pid) {
      proc.kill("SIGTERM");
    }
  }

  /**
   * Dispose pattern for VS Code extension subscriptions. Calls close().
   */
  dispose(): void {
    this.close();
  }

  // ── Private ──────────────────────────────────────────────────────────────

  private _setState(s: McpClientState): void {
    const old = this._state;
    this._state = s;
    this._emitter.emit("state_changed", { old_state: old, new_state: s });
  }

  private _cleanup(): void {
    if (this._rl) {
      this._rl.removeAllListeners();
      this._rl.close();
      this._rl = null;
    }
    if (this._process) {
      this._process.stdout?.removeAllListeners();
      this._process.stderr?.removeAllListeners();
      this._process.removeAllListeners();
    }
  }

  /**
   * Send a JSON-RPC 2.0 request and return a promise that resolves/rejects
   * when the matching response arrives or times out.
   */
  private _sendRawRequest(method: string, params?: unknown): Promise<McpResponse> {
    return new Promise<McpResponse>((resolve, reject) => {
      if (!this._process || !this._process.stdin || this._process.killed) {
        reject(new Error("MCP client is not connected"));
        return;
      }

      if (this._pending.size >= McpClient.MAX_PENDING) {
        reject(new Error("Too many pending requests"));
        return;
      }

      const id = this._nextId++;

      const timer = setTimeout(() => {
        this._pending.delete(id);
        reject(new Error(`Request timed out: ${method}`));
      }, McpClient.REQUEST_TIMEOUT_MS);

      this._pending.set(id, { resolve, reject, timer });

      const request = JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params: params ?? {},
      });

      try {
        this._process.stdin.write(request + "\n");
      } catch (err) {
        this._pending.delete(id);
        clearTimeout(timer);
        reject(new Error(`Failed to write to anvil mcp stdin: ${err instanceof Error ? err.message : String(err)}`));
      }
    });
  }

  /**
   * Send a JSON-RPC 2.0 notification (no `id`, no response expected).
   */
  private _sendNotification(method: string, params: unknown): void {
    if (!this._process?.stdin || this._process.killed) return;

    const notification = JSON.stringify({
      jsonrpc: "2.0",
      method,
      params: params ?? {},
    });

    try {
      this._process.stdin.write(notification + "\n");
    } catch {
      // Silently ignore — notifications are fire-and-forget
    }
  }

  /**
   * Handle an incoming JSON-RPC message (response or notification).
   */
  private _handleMessage(raw: string): void {
    let msg: Record<string, unknown>;
    try {
      msg = JSON.parse(raw);
    } catch {
      // Malformed JSON — ignore per robustness principle
      return;
    }

    // JSON-RPC 2.0 responses must have an `id`
    if (msg.id !== undefined && msg.id !== null) {
      this._handleResponse(msg);
    } else {
      this._handleIncomingNotification(msg);
    }
  }

  /**
   * Route a JSON-RPC response to the matching pending request.
   */
  private _handleResponse(msg: Record<string, unknown>): void {
    const id = msg.id as number | string;
    const entry = this._pending.get(id);
    if (!entry) return; // Unknown id — possibly timed out already

    clearTimeout(entry.timer);
    this._pending.delete(id);

    const response: McpResponse = {
      jsonrpc: (msg.jsonrpc as string) ?? "2.0",
      id,
      result: msg.result,
      error: msg.error as McpResponse["error"],
    };

    entry.resolve(response);
  }

  /**
   * Handle an incoming notification from the server and emit it to registered handlers.
   */
  private _handleIncomingNotification(msg: Record<string, unknown>): void {
    const method = msg.method as string;
    if (!method) return;

    const notification: McpNotification = {
      jsonrpc: (msg.jsonrpc as string) ?? "2.0",
      method,
      params: msg.params,
    };

    this._emitter.emit(method, notification.params);
    // Also emit on a generic 'notification' event for catch-all listeners
    this._emitter.emit("notification", notification);
  }
}
