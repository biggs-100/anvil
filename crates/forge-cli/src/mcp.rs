//! MCP (Model Context Protocol) server over stdin/stdout for AI agent integration.
// These allow(dead_code) suppress warnings on protocol types that exist for
// deserialization completeness and spec-defined error codes not yet used.
#![allow(dead_code)]
//!
//! Runs as the `forge mcp` subcommand. Implements the Model Context Protocol,
//! reading JSON-RPC 2.0 messages (line-delimited) from stdin, dispatching to
//! the appropriate handler, and writing JSON-RPC 2.0 responses to stdout.
//!
//! # MCP Lifecycle
//!
//! ```text
//! Client                          Server
//!   │                               │
//!   ├── initialize (request) ──────►│── validates version
//!   │                               │── returns ServerCapabilities
//!   │◄──── initialize (response) ───│
//!   │                               │
//!   ├── notifications/initialized ─►│── marks state as Initialized
//!   │                               │
//!   ├── tools/list ────────────────►│── returns 6 tool definitions
//!   │◄── tools/list (response) ────│
//!   │                               │
//!   ├── resources/read ────────────►│── calls McpExporter
//!   │◄── resources/read (response) ─│
//!   │                               │
//!   ├── shutdown (notification) ───►│── breaks loop, exits cleanly
//!   │                               │
//!   │◄── forge/state_changed ───────│── background EventBus subscriber
//!   │◄── forge/error ───────────────│
//! ```
//!
//! Before the initialize handshake completes, all non-initialize requests are
//! rejected with an error (protocol version not negotiated).

use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tokio::sync::broadcast;

use forge_core::operations::{RunOperation, ShellOperation, PlanOperation};
use forge_core::types::OperationResult;
use forge_core::{ContextExporter, Operation};
use forge_core::{Engine, Event, EventBus, ContextEngine, ContextOptions, McpExporter};
use forge_core::{DiagnosticEngine, DiagnosticContext, DiagnosticMode};

// ── MCP Error Codes ──────────────────────────────────────────────────────────

/// MCP / JSON-RPC 2.0 error code constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ResourceNotFound,
    ToolExecutionError,
    Custom(i64),
}

impl McpErrorCode {
    pub fn code(self) -> i64 {
        match self {
            McpErrorCode::ParseError => -32700,
            McpErrorCode::InvalidRequest => -32600,
            McpErrorCode::MethodNotFound => -32601,
            McpErrorCode::InvalidParams => -32602,
            McpErrorCode::InternalError => -32603,
            McpErrorCode::ResourceNotFound => -32000,
            McpErrorCode::ToolExecutionError => -32001,
            McpErrorCode::Custom(c) => c,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            McpErrorCode::ParseError => "Parse error",
            McpErrorCode::InvalidRequest => "Invalid request",
            McpErrorCode::MethodNotFound => "Method not found",
            McpErrorCode::InvalidParams => "Invalid params",
            McpErrorCode::InternalError => "Internal error",
            McpErrorCode::ResourceNotFound => "Resource not found",
            McpErrorCode::ToolExecutionError => "Tool execution error",
            McpErrorCode::Custom(_) => "Unknown error",
        }
    }
}

// ── MCP Message Types ────────────────────────────────────────────────────────

/// JSON-RPC 2.0 request envelope for MCP.
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[serde(default)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC 2.0 response envelope for MCP.
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

/// MCP error object inside a response.
#[derive(Debug, Serialize)]
struct McpError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP notification (no `id`) — sent from server or received from client.
#[derive(Debug, Serialize, Deserialize)]
struct McpNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

// ── Initialize Types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct InitializeParams {
    protocol_version: String,
    #[serde(default)]
    capabilities: ClientCapabilities,
    #[serde(default)]
    client_info: ClientInfo,
}

#[derive(Debug, Deserialize, Default)]
struct ClientCapabilities {
    #[serde(default)]
    roots: Option<Value>,
    #[serde(default)]
    sampling: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
struct ClientInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct InitializeResult {
    protocol_version: String,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
struct ServerCapabilities {
    resources: Value,
    tools: Value,
    prompts: Value,
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
}

// ── Tool Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct ListToolsResult {
    tools: Vec<ToolDescription>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolDescription {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct CallToolRequest {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

#[derive(Debug, Serialize)]
struct CallToolResult {
    content: Vec<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ToolResultContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ── Resource Types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ListResourcesResult {
    resources: Vec<ResourceDescription>,
}

#[derive(Debug, Serialize)]
struct ResourceDescription {
    uri: String,
    name: String,
    mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReadResourceRequest {
    uri: String,
}

#[derive(Debug, Serialize)]
struct ReadResourceResult {
    contents: Vec<ResourceContents>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceContents {
    uri: String,
    mime_type: String,
    text: String,
}

// ── Prompt Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ListPromptsResult {
    prompts: Vec<PromptDescription>,
}

#[derive(Debug, Serialize)]
struct PromptDescription {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Serialize)]
struct PromptArgument {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GetPromptRequest {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

#[derive(Debug, Serialize)]
struct GetPromptResult {
    messages: Vec<PromptMessage>,
}

#[derive(Debug, Serialize)]
struct PromptMessage {
    role: String,
    content: PromptContent,
}

#[derive(Debug, Serialize)]
struct PromptContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ── Server ───────────────────────────────────────────────────────────────────

/// Run the MCP server loop over stdin/stdout.
///
/// Reads line-delimited JSON-RPC 2.0 messages from stdin, dispatches to
/// handlers, and writes responses to stdout. An `Arc<AtomicBool>` tracks
/// whether the initialize handshake has completed.
pub async fn serve(current_dir: std::path::PathBuf) -> Result<(), String> {
    let engine = Arc::new(Engine::new(current_dir.clone())?);
    let event_bus = EventBus::new(100);
    let write_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    // Track initialized state — rejects all non-initialize before handshake
    let initialized = Arc::new(AtomicBool::new(false));

    // Spawn background notification subscriber
    let notif_write_lock = Arc::clone(&write_lock);
    let notif_rx = event_bus.subscribe();
    tokio::spawn(async move {
        notification_loop(notif_rx, notif_write_lock).await;
    });

    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;

        if n == 0 {
            // EOF — clean shutdown
            break;
        }

        let raw = line.trim().to_string();
        if raw.is_empty() {
            continue;
        }

        // Process requests sequentially (MCP lifecycle requires ordered dispatch)
        let response = dispatch(&engine, &current_dir, &raw, &initialized, &event_bus).await;
        if let Some(resp) = response {
            let json = match serde_json::to_string(&resp) {
                Ok(s) => s,
                Err(e) => format!(
                    r#"{{"jsonrpc":"2.0","id":null,"error":{{"code":-32603,"message":"Serialization error: {}" }}"#,
                    e.to_string().replace('"', "'")
                ),
            };
            let _guard = write_lock.lock().await;
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", json);
            let _ = out.flush();
        }
    }

    Ok(())
}

/// Background loop: subscribe to EventBus and emit MCP notifications.
async fn notification_loop(
    mut rx: broadcast::Receiver<Event>,
    write_lock: Arc<Mutex<()>>,
) {
    // Track last known engine state for forge/state_changed
    let mut last_state: Option<String> = None;

    loop {
        match rx.recv().await {
            Ok(event) => {
                // ── forge/state_changed: on lifecycle phase transitions ──
                let current = Some(event.runtime.clone());
                if last_state != current {
                    if let Some(ref old) = last_state {
                        let notif = McpNotification {
                            jsonrpc: "2.0".to_string(),
                            method: "forge/state_changed".to_string(),
                            params: json!({
                                "old_state": old,
                                "new_state": &event.runtime,
                            }),
                        };
                        if let Ok(json) = serde_json::to_string(&notif) {
                            let _guard = write_lock.lock().await;
                            let mut out = std::io::stdout().lock();
                            let _ = writeln!(out, "{}", json);
                            let _ = out.flush();
                        }
                    }
                    last_state = current;
                }

                // ── forge/error: on operation failure ──
                if let forge_core::types::EventStatus::Failed(ref err) = event.status {
                    let notif = McpNotification {
                        jsonrpc: "2.0".to_string(),
                        method: "forge/error".to_string(),
                        params: json!({
                            "operation": event.phase,
                            "error": err,
                        }),
                    };
                    if let Ok(json) = serde_json::to_string(&notif) {
                        let _guard = write_lock.lock().await;
                        let mut out = std::io::stdout().lock();
                        let _ = writeln!(out, "{}", json);
                        let _ = out.flush();
                    }
                }

                // ── forge/warning: on warning in diagnostics or low health ──
                if let Some(ref msg) = event.message {
                    if msg.to_lowercase().contains("warning")
                        || msg.to_lowercase().contains("degraded")
                        || msg.to_lowercase().contains("critical")
                    {
                        let notif = McpNotification {
                            jsonrpc: "2.0".to_string(),
                            method: "forge/warning".to_string(),
                            params: json!({
                                "finding": msg,
                                "severity": "WARNING",
                            }),
                        };
                        if let Ok(json) = serde_json::to_string(&notif) {
                            let _guard = write_lock.lock().await;
                            let mut out = std::io::stdout().lock();
                            let _ = writeln!(out, "{}", json);
                            let _ = out.flush();
                        }
                    }
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

/// Parse and dispatch a single MCP message line.
///
/// Returns `Some(McpResponse)` for requests (messages with an `id`),
/// or `None` for notifications (no `id` — per JSON-RPC 2.0).
async fn dispatch(
    engine: &Engine,
    current_dir: &Path,
    line: &str,
    initialized: &AtomicBool,
    event_bus: &EventBus,
) -> Option<McpResponse> {
    let request: McpRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(_) => {
            return Some(error_response(None, McpErrorCode::ParseError, "Parse error"));
        }
    };

    // Validate jsonrpc version
    if request.jsonrpc.as_deref() != Some("2.0") {
        return Some(error_response(
            request.id.clone(),
            McpErrorCode::InvalidRequest,
            "Invalid Request: jsonrpc must be 2.0",
        ));
    }

    // Notifications (no id) produce no response
    let id = match request.id {
        Some(ref id_val) => {
            if id_val.is_null() {
                return Some(error_response(
                    None,
                    McpErrorCode::InvalidRequest,
                    "Invalid Request: id must be non-null or absent",
                ));
            }
            Some(id_val.clone())
        }
        None => {
            // Notification — handle without producing a response
            handle_notification(&request.method, &request.params.unwrap_or(Value::Null), initialized);
            return None;
        }
    };

    // MCP lifecycle guard: only `initialize` is allowed before initialization
    if !initialized.load(Ordering::Acquire) && request.method != "initialize" {
        return Some(error_response(
            id,
            McpErrorCode::InvalidRequest,
            "Server not initialized. Send initialize request first.",
        ));
    }

    let params = request.params.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        // ── Lifecycle ──────────────────────────────────────────────────
        "initialize" => handle_initialize(engine, &params, initialized).await,
        "notifications/initialized" => {
            // Already handled as notification above, but response variant
            Ok(json!({}))
        }

        // ── Tools ──────────────────────────────────────────────────────
        "tools/list" => handle_list_tools().await,
        "tools/call" => handle_call_tool(engine, current_dir, &params, event_bus).await,

        // ── Resources ──────────────────────────────────────────────────
        "resources/list" => handle_list_resources().await,
        "resources/read" => handle_read_resource(current_dir, &params).await,

        // ── Prompts ────────────────────────────────────────────────────
        "prompts/list" => handle_list_prompts().await,
        "prompts/get" => handle_get_prompt(engine, &params).await,

        _ => Err(format!("Method not found: {}", request.method)),
    };

    match result {
        Ok(value) => Some(success_response(id, value)),
        Err(msg) => {
            if msg.starts_with("Method not found") {
                Some(error_response(id, McpErrorCode::MethodNotFound, &msg))
            } else {
                Some(error_response(id, McpErrorCode::InternalError, &msg))
            }
        }
    }
}

/// Handle incoming notifications (messages without `id`).
fn handle_notification(method: &str, _params: &Value, initialized: &AtomicBool) {
    match method {
        "notifications/initialized" => {
            initialized.store(true, Ordering::Release);
        }
        "shutdown" => {
            // The server loop will break on EOF; this is handled by the client
            // disconnecting. We mark shutdown but continue reading.
            // For MCP shutdown, the client sends a notification and then closes.
        }
        _ => {
            // Unknown notification — silently ignored per JSON-RPC spec
        }
    }
}

// ── Response Helpers ─────────────────────────────────────────────────────────

fn success_response(id: Option<Value>, result: Value) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: McpErrorCode, message: &str) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(McpError {
            code: code.code(),
            message: message.to_string(),
            data: None,
        }),
    }
}

// ── Handler: initialize ──────────────────────────────────────────────────────

async fn handle_initialize(
    _engine: &Engine,
    params: &Value,
    initialized: &AtomicBool,
) -> Result<Value, String> {
    let _init_params: InitializeParams = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid initialize params: {}", e))?;

    // Supported protocol version
    let protocol_version = "2024-11-05".to_string();

    let result = InitializeResult {
        protocol_version,
        capabilities: ServerCapabilities {
            resources: json!({
                "subscribe": false,
                "listChanged": false,
            }),
            tools: json!({
                "listChanged": false,
            }),
            prompts: json!({
                "listChanged": false,
            }),
        },
        server_info: ServerInfo {
            name: "forge".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    // Mark as initialized — we accept the version
    initialized.store(true, Ordering::Release);

    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize InitializeResult: {}", e))
}

// ── Handler: tools ───────────────────────────────────────────────────────────

async fn handle_list_tools() -> Result<Value, String> {
    let tools = vec![
        ToolDescription {
            name: "forge_run".to_string(),
            description: "Execute a command inside the activated forge environment".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cmd": { "type": "string", "description": "Command to execute" },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Arguments to pass to the command"
                    }
                },
                "required": ["cmd"]
            }),
        },
        ToolDescription {
            name: "forge_shell".to_string(),
            description: "Spawn an interactive subshell inside the forge environment".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDescription {
            name: "forge_sync".to_string(),
            description: "Sync all configured runtimes from the lockfile".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDescription {
            name: "forge_plan".to_string(),
            description: "Show the planned operations without executing them".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDescription {
            name: "forge_explain".to_string(),
            description: "Explain the resolved configuration and cache state for a runtime".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "runtime": { "type": "string", "description": "Runtime name to explain (e.g. node, python)" }
                },
                "required": ["runtime"]
            }),
        },
        ToolDescription {
            name: "forge_doctor".to_string(),
            description: "Run diagnostics checks on the forge environment".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "description": "Diagnostic mode: 'quick' for fast checks, 'deep' for full verification",
                        "enum": ["quick", "deep"]
                    }
                }
            }),
        },
    ];

    let result = ListToolsResult { tools };
    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize tools list: {}", e))
}

async fn handle_call_tool(
    engine: &Engine,
    current_dir: &Path,
    params: &Value,
    event_bus: &EventBus,
) -> Result<Value, String> {
    let request: CallToolRequest = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid call_tool params: {}", e))?;

    let (content, is_error) = match request.name.as_str() {
        "forge_run" => {
            let cmd = request.arguments
                .as_ref()
                .and_then(|a| a.get("cmd"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: cmd".to_string())?;
            let args: Vec<String> = request.arguments
                .as_ref()
                .and_then(|a| a.get("args"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            engine.sync().await?;

            let mut ctx = forge_core::operations::Context::new(
                current_dir.to_path_buf(),
                forge_core::get_cache_dir()?,
                event_bus.clone(),
            );
            let _ = ctx.load_config();
            let _ = ctx.load_lockfile();

            let op = RunOperation { cmd: cmd.to_string(), args };
            let plan = op.plan(&ctx)?;
            let result: OperationResult = op.execute(&mut ctx, plan).await?;
            let is_err = result.status == forge_core::types::OperationStatus::Failure;
            let text = serde_json::to_string_pretty(&result)
                .map_err(|e| format!("Failed to serialize run result: {}", e))?;
            (text, is_err)
        }
        "forge_shell" => {
            engine.sync().await?;

            let mut ctx = forge_core::operations::Context::new(
                current_dir.to_path_buf(),
                forge_core::get_cache_dir()?,
                event_bus.clone(),
            );
            let _ = ctx.load_config();
            let _ = ctx.load_lockfile();

            let op = ShellOperation;
            let plan = op.plan(&ctx)?;
            let result: OperationResult = op.execute(&mut ctx, plan).await?;
            let is_err = result.status == forge_core::types::OperationStatus::Failure;
            let text = serde_json::to_string_pretty(&result)
                .map_err(|e| format!("Failed to serialize shell result: {}", e))?;
            (text, is_err)
        }
        "forge_sync" => {
            engine.sync().await?;
            (r#"{"status":"ok"}"#.to_string(), false)
        }
        "forge_plan" => {
            let mut ctx = forge_core::operations::Context::new(
                current_dir.to_path_buf(),
                forge_core::get_cache_dir()?,
                event_bus.clone(),
            );
            let _ = ctx.load_config();
            let _ = ctx.load_lockfile();

            let op = PlanOperation;
            let plan = op.plan(&ctx)?;
            let result: OperationResult = op.execute(&mut ctx, plan).await?;
            let is_err = result.status == forge_core::types::OperationStatus::Failure;
            let text = serde_json::to_string_pretty(&result)
                .map_err(|e| format!("Failed to serialize plan result: {}", e))?;
            (text, is_err)
        }
        "forge_explain" => {
            let runtime = request.arguments
                .as_ref()
                .and_then(|a| a.get("runtime"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: runtime".to_string())?;
            let explanation = engine.explain(runtime).await?;
            let text = serde_json::to_string_pretty(&explanation)
                .map_err(|e| format!("Failed to serialize explanation: {}", e))?;
            (text, false)
        }
        "forge_doctor" => {
            let mode_str = request.arguments
                .as_ref()
                .and_then(|a| a.get("mode"))
                .and_then(|v| v.as_str())
                .unwrap_or("quick");
            let mode = match mode_str {
                "deep" => DiagnosticMode::Deep,
                _ => DiagnosticMode::Fast,
            };

            let cache_dir = forge_core::get_cache_dir()?;
            let diag_ctx = DiagnosticContext {
                workspace_root: current_dir.to_path_buf(),
                cache_dir,
                mode,
                active_profile: None,
            };

            let diagnostic_engine = DiagnosticEngine::new();
            let report = diagnostic_engine.run(&diag_ctx).await;
            let is_err = report.health_score < 70;
            let text = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize diagnostic report: {}", e))?;
            (text, is_err)
        }
        _ => {
            return Err(format!("Tool not found: {}", request.name));
        }
    };

    let result = CallToolResult {
        content: vec![ToolResultContent {
            content_type: "text".to_string(),
            text: content,
        }],
        is_error: if is_error { Some(true) } else { None },
    };

    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize tool result: {}", e))
}

// ── Handler: resources ───────────────────────────────────────────────────────

async fn handle_list_resources() -> Result<Value, String> {
    let result = ListResourcesResult {
        resources: vec![ResourceDescription {
            uri: "forge://context/active".to_string(),
            name: "Active Forge Context".to_string(),
            mime_type: "application/json".to_string(),
            description: Some("Complete serialized Forge environment context including runtimes, configuration, diagnostics, workspace, environment, and secrets metadata".to_string()),
        }],
    };
    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize resource list: {}", e))
}

async fn handle_read_resource(current_dir: &Path, params: &Value) -> Result<Value, String> {
    let request: ReadResourceRequest = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid read_resource params: {}", e))?;

    if request.uri != "forge://context/active" {
        return Err(format!("Resource not found: {}", request.uri));
    }

    // Build context engine (same pattern as jsonrpc.rs handle_context_get)
    let mut ctx_engine = ContextEngine::new();
    ctx_engine.register(std::sync::Arc::new(forge_core::RuntimeProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::ConfigurationProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::DiagnosticsProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::WorkspaceProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::EnvironmentProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::SecretsProviderImpl));

    let cache_dir = forge_core::get_cache_dir()?;
    let options = ContextOptions {
        scopes: Vec::new(),
        excludes: Vec::new(),
        workspace_root: current_dir.to_path_buf(),
        cache_dir,
        active_profile: None,
    };

    let context = ctx_engine.query(&options).await?;

    // Use McpExporter which returns the MCP envelope format
    let exporter = McpExporter;
    let exported = exporter.export(&context)?;

    // Parse the McpExporter output — it returns a JSON string like:
    // { "contents": [{ "uri": "forge://context/active", "mimeType": "application/json", "text": "..." }] }
    // We need to return ReadResourceResult with contents array
    let exported_value: Value = serde_json::from_str(&exported)
        .map_err(|e| format!("Failed to parse McpExporter output: {}", e))?;

    let contents = exported_value
        .get("contents")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let result = ReadResourceResult {
        contents: contents
            .into_iter()
            .map(|item| ResourceContents {
                uri: item.get("uri").and_then(|u| u.as_str()).unwrap_or("forge://context/active").to_string(),
                mime_type: item.get("mimeType").and_then(|m| m.as_str()).unwrap_or("application/json").to_string(),
                text: item.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            })
            .collect(),
    };

    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize ReadResourceResult: {}", e))
}

// ── Handler: prompts ─────────────────────────────────────────────────────────

async fn handle_list_prompts() -> Result<Value, String> {
    let result = ListPromptsResult {
        prompts: vec![
            PromptDescription {
                name: "forge:status".to_string(),
                description: "Current forge environment state summary including lifecycle status, runtimes, and health".to_string(),
                arguments: None,
            },
            PromptDescription {
                name: "forge:diagnose".to_string(),
                description: "Run diagnostics and return a health report with findings, explanations, and suggested fixes".to_string(),
                arguments: None,
            },
            PromptDescription {
                name: "forge:explain".to_string(),
                description: "Explain the resolved configuration for a specific runtime in markdown format".to_string(),
                arguments: None,
            },
        ],
    };
    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize prompt list: {}", e))
}

async fn handle_get_prompt(engine: &Engine, params: &Value) -> Result<Value, String> {
    let request: GetPromptRequest = serde_json::from_value(params.clone())
        .map_err(|e| format!("Invalid get_prompt params: {}", e))?;

    let markdown = match request.name.as_str() {
        "forge:status" => {
            let state = engine.get_status().await?;
            format!(
                "# Forge Environment Status\n\n\
                 ## Lifecycle State\n\
                 {}\n\n\
                 ## Overview\n\
                 The forge environment manages development runtimes and configuration. \
                 Run `forge doctor` for detailed health checks.\n",
                state,
            )
        }
        "forge:diagnose" => {
            let cache_dir = forge_core::get_cache_dir()?;
            let workspace_root = engine.workspace_root.clone();
            let diag_ctx = DiagnosticContext {
                workspace_root,
                cache_dir,
                mode: DiagnosticMode::Fast,
                active_profile: None,
            };
            let diagnostic_engine = DiagnosticEngine::new();
            let report = diagnostic_engine.run(&diag_ctx).await;

            let mut md = String::new();
            md.push_str("# Forge Diagnostics Report\n\n");
            md.push_str(&format!("**Health Score**: {}/100\n\n", report.health_score));
            md.push_str(&format!("**Mode**: {:?} | **Elapsed**: {}ms\n\n", report.mode, report.elapsed_ms));

            if report.findings.is_empty() {
                md.push_str("No issues found. Environment is healthy.\n");
            } else {
                md.push_str("## Findings\n\n");
                for f in &report.findings {
                    let sev = format!("{:?}", f.severity);
                    md.push_str(&format!("- **{}** [{}] {} (Confidence: {}%)\n", f.code, sev, f.message, f.confidence));
                    md.push_str(&format!("  - *What*: {}\n", f.explanation.what));
                    md.push_str(&format!("  - *Why*: {}\n", f.explanation.why));
                    md.push_str(&format!("  - *How*: {}\n", f.explanation.how));
                    if let Some(ref qf) = f.suggested_quick_fix {
                        md.push_str(&format!("  - Suggested fix: {}\n", qf.description));
                    }
                    md.push('\n');
                }
            }
            md
        }
        "forge:explain" => {
            let runtime = request.arguments
                .as_ref()
                .and_then(|a| a.get("runtime"))
                .and_then(|v| v.as_str())
                .unwrap_or("all");

            if runtime == "all" {
                "Please specify a runtime name to explain (e.g., `node`, `python`).".to_string()
            } else {
                match engine.explain(runtime).await {
                    Ok(explanation) => {
                        let mut md = String::new();
                        md.push_str(&format!("# Runtime: {}\n\n", explanation.runtime));
                        md.push_str(&format!("**State**: {}\n\n", explanation.state));
                        if !explanation.diagnostics.is_empty() {
                            md.push_str("## Diagnostics\n\n");
                            for d in &explanation.diagnostics {
                                md.push_str(&format!("- {}\n", d));
                            }
                        }
                        md
                    }
                    Err(e) => {
                        format!("# Error\n\nFailed to explain runtime `{}`: {}", runtime, e)
                    }
                }
            }
        }
        _ => {
            return Err(format!("Prompt not found: {}", request.name));
        }
    };

    let result = GetPromptResult {
        messages: vec![PromptMessage {
            role: "assistant".to_string(),
            content: PromptContent {
                content_type: "text".to_string(),
                text: markdown,
            },
        }],
    };

    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize prompt result: {}", e))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Phase 1: Message serialization roundtrips ──────────────────────

    #[test]
    fn test_mcp_error_code_values() {
        assert_eq!(McpErrorCode::ParseError.code(), -32700);
        assert_eq!(McpErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(McpErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(McpErrorCode::InvalidParams.code(), -32602);
        assert_eq!(McpErrorCode::InternalError.code(), -32603);
        assert_eq!(McpErrorCode::ResourceNotFound.code(), -32000);
        assert_eq!(McpErrorCode::ToolExecutionError.code(), -32001);
    }

    #[test]
    fn test_mcp_error_code_messages() {
        assert_eq!(McpErrorCode::ParseError.message(), "Parse error");
        assert_eq!(McpErrorCode::MethodNotFound.message(), "Method not found");
    }

    #[test]
    fn test_success_response_serialization() {
        let resp = success_response(Some(json!(1)), json!({"status": "ok"}));
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"result\""));
        assert!(json_str.contains("\"status\":\"ok\""));
        assert!(!json_str.contains("\"error\""));
    }

    #[test]
    fn test_error_response_serialization() {
        let resp = error_response(Some(json!(1)), McpErrorCode::ParseError, "Parse error");
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("\"code\":-32700"));
        assert!(json_str.contains("\"message\":\"Parse error\""));
        assert!(json_str.contains("\"id\":1"));
        assert!(!json_str.contains("\"result\""));
    }

    #[test]
    fn test_error_response_no_id() {
        let resp = error_response(None, McpErrorCode::MethodNotFound, "Method not found");
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(!json_str.contains("\"id\""));
        assert!(json_str.contains("\"code\":-32601"));
    }

    #[test]
    fn test_mcp_request_deserialize() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#;
        let req: McpRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.jsonrpc.as_deref(), Some("2.0"));
        assert_eq!(req.id, Some(json!(1)));
        assert_eq!(req.method, "initialize");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_mcp_notification_no_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
        let req: McpRequest = serde_json::from_str(raw).unwrap();
        assert!(req.id.is_none());
        assert_eq!(req.method, "notifications/initialized");
    }

    #[test]
    fn test_initialize_result_serialization() {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                resources: json!({"subscribe": false, "listChanged": false}),
                tools: json!({"listChanged": false}),
                prompts: json!({"listChanged": false}),
            },
            server_info: ServerInfo {
                name: "forge".to_string(),
                version: "0.1.0".to_string(),
            },
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"protocol_version\":\"2024-11-05\""));
        assert!(json_str.contains("\"server_info\""));
        assert!(json_str.contains("\"forge\""));
    }

    #[test]
    fn test_list_tools_serialization() {
        let result = ListToolsResult {
            tools: vec![
                ToolDescription {
                    name: "forge_run".to_string(),
                    description: "Execute a command".to_string(),
                    input_schema: json!({"type": "object", "properties": {}}),
                },
            ],
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"name\":\"forge_run\""));
        assert!(json_str.contains("\"input_schema\""));
        assert!(json_str.contains("\"tools\""));
    }

    #[test]
    fn test_list_tools_six_tools() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(handle_list_tools()).unwrap();
        let result: ListToolsResult = serde_json::from_value(result).unwrap();
        assert_eq!(result.tools.len(), 6);
        let names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"forge_run"));
        assert!(names.contains(&"forge_shell"));
        assert!(names.contains(&"forge_sync"));
        assert!(names.contains(&"forge_plan"));
        assert!(names.contains(&"forge_explain"));
        assert!(names.contains(&"forge_doctor"));
    }

    #[test]
    fn test_call_tool_request_deserialize() {
        let raw = r#"{"name":"forge_run","arguments":{"cmd":"echo","args":["hello"]}}"#;
        let req: CallToolRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.name, "forge_run");
        let cmd = req.arguments.as_ref().and_then(|a| a.get("cmd")).and_then(|v| v.as_str());
        assert_eq!(cmd, Some("echo"));
    }

    #[test]
    fn test_call_tool_result_serialization() {
        let result = CallToolResult {
            content: vec![ToolResultContent {
                content_type: "text".to_string(),
                text: "hello".to_string(),
            }],
            is_error: None,
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"type\":\"text\""));
        assert!(json_str.contains("\"text\":\"hello\""));
    }

    #[test]
    fn test_read_resource_request_deserialize() {
        let raw = r#"{"uri":"forge://context/active"}"#;
        let req: ReadResourceRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.uri, "forge://context/active");
    }

    #[test]
    fn test_read_resource_result_serialization() {
        let result = ReadResourceResult {
            contents: vec![ResourceContents {
                uri: "forge://context/active".to_string(),
                mime_type: "application/json".to_string(),
                text: "{}".to_string(),
            }],
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"uri\":\"forge://context/active\""));
        assert!(json_str.contains("\"mime_type\":\"application/json\""));
    }

    #[test]
    fn test_list_resources_serialization() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(handle_list_resources()).unwrap();
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"forge://context/active\""));
    }

    #[test]
    fn test_list_prompts_serialization() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(handle_list_prompts()).unwrap();
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("forge:status"));
        assert!(json_str.contains("forge:diagnose"));
        assert!(json_str.contains("forge:explain"));
    }

    #[test]
    fn test_get_prompt_request_deserialize() {
        let raw = r#"{"name":"forge:status"}"#;
        let req: GetPromptRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.name, "forge:status");
    }

    #[test]
    fn test_get_prompt_result_serialization() {
        let result = GetPromptResult {
            messages: vec![PromptMessage {
                role: "assistant".to_string(),
                content: PromptContent {
                    content_type: "text".to_string(),
                    text: "# Status\n\nEverything ok".to_string(),
                },
            }],
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"role\":\"assistant\""));
        assert!(json_str.contains("\"type\":\"text\""));
        assert!(json_str.contains("Everything ok"));
    }

    #[test]
    fn test_mcp_notification_serialization() {
        let notif = McpNotification {
            jsonrpc: "2.0".to_string(),
            method: "forge/state_changed".to_string(),
            params: json!({"old_state": "Locked", "new_state": "Ready"}),
        };
        let json_str = serde_json::to_string(&notif).unwrap();
        assert!(json_str.contains("\"method\":\"forge/state_changed\""));
        assert!(json_str.contains("\"params\""));
        assert!(!json_str.contains("\"id\"")); // notifications have no id
    }

    #[test]
    fn test_mcp_notification_deserialize() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
        let notif: McpNotification = serde_json::from_str(raw).unwrap();
        assert_eq!(notif.method, "notifications/initialized");
        assert_eq!(notif.jsonrpc, "2.0");
    }

    // ── Response helper tests ──────────────────────────────────────────

    #[test]
    fn test_success_response_omits_error_on_null_result() {
        let resp = success_response(Some(json!(1)), Value::Null);
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("\"result\":null"));
        assert!(!json_str.contains("\"error\""));
    }

    #[test]
    fn test_error_response_omits_result() {
        let resp = error_response(None, McpErrorCode::InternalError, "Internal error");
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(!json_str.contains("\"result\""));
        assert!(json_str.contains("\"error\""));
    }

    #[test]
    fn test_call_tool_result_with_error_flag() {
        let result = CallToolResult {
            content: vec![ToolResultContent {
                content_type: "text".to_string(),
                text: "error".to_string(),
            }],
            is_error: Some(true),
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"is_error\":true"));
    }

    #[test]
    fn test_tool_description_with_input_schema() {
        let tool = ToolDescription {
            name: "forge_run".to_string(),
            description: "Run a command".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cmd": { "type": "string" }
                },
                "required": ["cmd"]
            }),
        };
        let json_str = serde_json::to_string(&tool).unwrap();
        assert!(json_str.contains("\"input_schema\""));
        assert!(json_str.contains("\"cmd\""));
        assert!(json_str.contains("\"required\""));
    }

    #[test]
    fn test_resource_content_with_optional_fields() {
        let content = ResourceContents {
            uri: "forge://context/active".to_string(),
            mime_type: "text/plain".to_string(),
            text: "data".to_string(),
        };
        let json_str = serde_json::to_string(&content).unwrap();
        assert_eq!(
            serde_json::from_str::<ResourceContents>(&json_str).unwrap().uri,
            "forge://context/active"
        );
    }

    #[test]
    fn test_prompt_message_structure() {
        let msg = PromptMessage {
            role: "user".to_string(),
            content: PromptContent {
                content_type: "text".to_string(),
                text: "Hello".to_string(),
            },
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        assert!(json_str.contains("\"role\":\"user\""));
        assert!(json_str.contains("\"type\":\"text\""));
    }
}
