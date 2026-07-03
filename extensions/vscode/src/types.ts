/**
 * TypeScript interfaces mirroring the MCP types from anvil's `crates/anvil-cli/src/mcp.rs`.
 *
 * These represent the JSON-RPC 2.0 wire format that the anvil MCP server speaks
 * over line-delimited stdin/stdout.
 */

/** JSON-RPC 2.0 request envelope. */
export interface McpRequest {
  jsonrpc: string;
  id?: number | string | null;
  method: string;
  params?: unknown;
}

/** JSON-RPC 2.0 response envelope. */
export interface McpResponse {
  jsonrpc: string;
  id?: number | string | null;
  result?: unknown;
  error?: McpError;
}

/** JSON-RPC 2.0 error object. */
export interface McpError {
  code: number;
  message: string;
  data?: unknown;
}

/** MCP notification (message without `id`). */
export interface McpNotification {
  jsonrpc: string;
  method: string;
  params: unknown;
}

/** Result from a tools/call request. */
export interface CallToolResult {
  content: ToolResultContent[];
  is_error?: boolean;
}

/** Single content item inside a CallToolResult. */
export interface ToolResultContent {
  type: string;
  text: string;
}

/** Result from a prompts/get request. */
export interface GetPromptResult {
  messages: PromptMessage[];
}

/** Single message inside a GetPromptResult. */
export interface PromptMessage {
  role: string;
  content: PromptContent;
}

/** Content item inside a PromptMessage. */
export interface PromptContent {
  type: string;
  text: string;
}

/** Result type for tools/list. */
export interface ListToolsResult {
  tools: ToolDescription[];
}

/** Description of a single tool. */
export interface ToolDescription {
  name: string;
  description: string;
  input_schema: unknown;
}

/** Notification from anvil event bus (anvil/warning, anvil/error, anvil/state_changed). */
export interface AnvilWarningNotification {
  method: "anvil/warning";
  params: { finding: string; severity: "WARNING" };
}

export interface AnvilErrorNotification {
  method: "anvil/error";
  params: { operation: string; error: string };
}

export interface AnvilStateChangedNotification {
  method: "anvil/state_changed";
  params: { old_state: string; new_state: string };
}

export type AnvilNotification =
  | AnvilWarningNotification
  | AnvilErrorNotification
  | AnvilStateChangedNotification;

/** MCP client connection states. */
export type McpClientState =
  | "connecting"
  | "ready"
  | "disconnected"
  | "error";
