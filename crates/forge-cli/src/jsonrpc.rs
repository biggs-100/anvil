//! JSON-RPC 2.0 server over stdin/stdout for SDK transport.
//!
//! Runs as the `forge jsonrpc` subcommand. Reads one JSON-RPC request per line
//! from stdin, dispatches to the appropriate handler, and writes one JSON-RPC
//! response per line to stdout. Each request spawns a tokio task; responses
//! are written via `println!` which is internally synchronised.

use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use forge_core::operations::{RunOperation, ShellOperation};
use forge_core::types::OperationResult;
use forge_core::{ContextExporter, Engine, Operation};

// ── JSON-RPC 2.0 Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Request {
    #[serde(default)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// ── JSON-RPC 2.0 Error Codes ────────────────────────────────────────────────

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INTERNAL_ERROR: i64 = -32603;

/// Max custom error code start.
const _CUSTOM_ERROR: i64 = -32000;

// ── Server ──────────────────────────────────────────────────────────────────

/// Run the JSON-RPC 2.0 server loop.
///
/// Reads lines from stdin, spawns a tokio task per request, and writes
/// responses to stdout. A shared mutex guards stdout writes so lines
/// from concurrent tasks do not interleave.
pub async fn serve(current_dir: std::path::PathBuf) -> Result<(), String> {
    let engine = Arc::new(Engine::new(current_dir.clone())?);
    // Mutex<()> guards stdout — the lock is held only during the sync write.
    let write_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

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

        // Process sequentially — simpler and avoids ordering issues
        let response = dispatch(&engine, &current_dir, &raw).await;
        if let Some(resp) = response {
            let json = match serde_json::to_string(&resp) {
                Ok(s) => s,
                Err(e) => format!(
                    r#"{{"jsonrpc":"2.0","id":null,"error":{{"code":-32603,"message":{} }}"#,
                    serde_json::to_string(&format!("Serialization error: {}", e))
                        .unwrap_or_else(|_| "\"Internal error\"".to_string())
                ),
            };
            // Acquire write lock, write, drop lock
            let _guard = write_lock.lock().await;
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", json);
            let _ = out.flush();
        }
    }

    Ok(())
}

/// Parse and dispatch a single JSON-RPC request line.
///
/// Returns `None` for notification requests (no `id`), which MUST NOT
/// produce a response per JSON-RPC 2.0 spec.
async fn dispatch(engine: &Engine, current_dir: &Path, line: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(_) => {
            return Some(error_response(None, PARSE_ERROR, "Parse error"));
        }
    };

    // Validate jsonrpc version
    if request.jsonrpc.as_deref() != Some("2.0") {
        return Some(error_response(request.id.clone(), INVALID_REQUEST, "Invalid Request"));
    }

    // Notifications (no id) produce no response
    let id = match request.id {
        Some(ref id_val) => {
            if id_val.is_null() {
                return Some(error_response(None, INVALID_REQUEST, "Invalid Request: id must be non-null or absent"));
            }
            Some(id_val.clone())
        }
        None => return None, // notification — no response
    };

    let params = request.params.unwrap_or(Value::Null);

    let result = match request.method.as_str() {
        // ── engine.* ──────────────────────────────────────────────────
        "engine.status" => handle_engine_status(engine).await,
        "engine.sync" => handle_engine_sync(engine).await,
        "engine.repair" => handle_engine_repair(engine).await,
        "engine.clean" => handle_engine_clean(engine).await,
        "engine.explain" => handle_engine_explain(engine, &params).await,
        "engine.history" => handle_engine_history(engine, &params).await,

        // ── exec.* ────────────────────────────────────────────────────
        "exec.run" => handle_exec_run(current_dir, &params).await,
        "exec.shell" => handle_exec_shell(current_dir, &params).await,

        // ── env.* ─────────────────────────────────────────────────────
        "env.list" => handle_env_list(engine).await,
        "env.get" => handle_env_get(engine, &params).await,
        "env.set" => handle_env_set(engine, &params).await,
        "env.unset" => handle_env_unset(engine, &params).await,
        "env.resolve" => handle_env_resolve(engine, &params).await,

        // ── secret.* ──────────────────────────────────────────────────
        "secret.set" => handle_secret_set(engine, &params).await,
        "secret.get" => handle_secret_get(engine, &params).await,
        "secret.list" => handle_secret_list(engine).await,
        "secret.remove" => handle_secret_remove(engine, &params).await,

        // ── context.* ─────────────────────────────────────────────────
        "context.get" => handle_context_get(current_dir, &params).await,

        _ => Err(format!("Method not found: {}", request.method)),
    };

    match result {
        Ok(value) => Some(success_response(id, value)),
        Err(msg) => {
            if msg.starts_with("Method not found") {
                Some(error_response(id, METHOD_NOT_FOUND, &msg))
            } else {
                Some(error_response(id, INTERNAL_ERROR, &msg))
            }
        }
    }
}

// ── Response Helpers ────────────────────────────────────────────────────────

fn success_response(id: Option<Value>, result: Value) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}

// ── Handler: engine.* ───────────────────────────────────────────────────────

async fn handle_engine_status(engine: &Engine) -> Result<Value, String> {
    let state = engine.get_status().await?;
    Ok(json!({ "state": state }))
}

async fn handle_engine_sync(engine: &Engine) -> Result<Value, String> {
    engine.sync().await?;
    Ok(json!({}))
}

async fn handle_engine_repair(engine: &Engine) -> Result<Value, String> {
    engine.repair().await?;
    Ok(json!({}))
}

async fn handle_engine_clean(engine: &Engine) -> Result<Value, String> {
    engine.clean().await?;
    Ok(json!({}))
}

async fn handle_engine_explain(engine: &Engine, params: &Value) -> Result<Value, String> {
    let runtime = params
        .get("runtime")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: runtime".to_string())?;
    let explanation = engine.explain(runtime).await?;
    serde_json::to_value(&explanation)
        .map_err(|e| format!("Failed to serialize explanation: {}", e))
}

async fn handle_engine_history(engine: &Engine, params: &Value) -> Result<Value, String> {
    let limit = params.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
    let history = engine.history(limit).await?;
    serde_json::to_value(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))
}

// ── Handler: exec.* ─────────────────────────────────────────────────────────

async fn handle_exec_run(current_dir: &Path, params: &Value) -> Result<Value, String> {
    let cmd = params
        .get("cmd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: cmd".to_string())?
        .to_string();
    let args: Vec<String> = params
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let event_bus = forge_core::event_bus::EventBus::new(100);
    let cache_dir = forge_core::get_cache_dir()?;
    let workspace_root = forge_core::find_forge_toml(current_dir)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| current_dir.to_path_buf());
    let mut ctx = forge_core::operations::Context::new(workspace_root.clone(), cache_dir, event_bus.clone());
    let _ = ctx.load_config();
    let _ = ctx.load_lockfile();

    let op = RunOperation { cmd: cmd.clone(), args: args.clone() };
    let plan = op.plan(&ctx)?;
    let result: OperationResult = op.execute(&mut ctx, plan).await?;
    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize run result: {}", e))
}

async fn handle_exec_shell(current_dir: &Path, _params: &Value) -> Result<Value, String> {
    let event_bus = forge_core::event_bus::EventBus::new(100);
    let cache_dir = forge_core::get_cache_dir()?;
    let workspace_root = forge_core::find_forge_toml(current_dir)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| current_dir.to_path_buf());
    let mut ctx = forge_core::operations::Context::new(workspace_root.clone(), cache_dir, event_bus.clone());
    let _ = ctx.load_config();
    let _ = ctx.load_lockfile();

    let op = ShellOperation;
    let plan = op.plan(&ctx)?;
    let result: OperationResult = op.execute(&mut ctx, plan).await?;
    serde_json::to_value(&result)
        .map_err(|e| format!("Failed to serialize shell result: {}", e))
}

// ── Handler: env.* ──────────────────────────────────────────────────────────

async fn handle_env_list(engine: &Engine) -> Result<Value, String> {
    let vars = engine.env_list().await?;
    serde_json::to_value(&vars)
        .map_err(|e| format!("Failed to serialize env list: {}", e))
}

async fn handle_env_get(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?;
    let val = engine.env_get(key).await?;
    Ok(json!(val))
}

async fn handle_env_set(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?
        .to_string();
    let value = params
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: value".to_string())?
        .to_string();
    engine.env_set(&key, &value).await?;
    Ok(json!({}))
}

async fn handle_env_unset(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?;
    engine.env_unset(key).await?;
    Ok(json!({}))
}

async fn handle_env_resolve(engine: &Engine, params: &Value) -> Result<Value, String> {
    let profile = params.get("profile").and_then(|v| v.as_str());
    let resolved = engine.env_resolve(profile).await?;
    serde_json::to_value(&resolved)
        .map_err(|e| format!("Failed to serialize resolved environment: {}", e))
}

// ── Handler: secret.* ───────────────────────────────────────────────────────

async fn handle_secret_set(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?
        .to_string();
    let value = params
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: value".to_string())?
        .to_string();
    engine.secret_set(&key, &value).await?;
    Ok(json!({}))
}

async fn handle_secret_get(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?;
    let val = engine.secret_get(key).await?;
    Ok(json!(val))
}

async fn handle_secret_list(engine: &Engine) -> Result<Value, String> {
    let keys = engine.secret_list().await?;
    Ok(json!(keys))
}

async fn handle_secret_remove(engine: &Engine, params: &Value) -> Result<Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: key".to_string())?;
    engine.secret_remove(key).await?;
    Ok(json!({}))
}

// ── Handler: context.* ──────────────────────────────────────────────────────

async fn handle_context_get(current_dir: &Path, params: &Value) -> Result<Value, String> {
    let format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("json")
        .to_string();
    let scope: Vec<String> = params
        .get("scope")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let exclude: Vec<String> = params
        .get("exclude")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let mut ctx_engine = forge_core::ContextEngine::new();
    ctx_engine.register(std::sync::Arc::new(forge_core::RuntimeProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::ConfigurationProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::DiagnosticsProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::WorkspaceProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::EnvironmentProviderImpl));
    ctx_engine.register(std::sync::Arc::new(forge_core::SecretsProviderImpl));

    let cache_dir = forge_core::get_cache_dir()?;
    let toml_path = forge_core::find_forge_toml(current_dir);
    let active_profile = if let Some(ref path) = toml_path {
        std::env::var("FORGE_PROFILE").ok().or_else(|| {
            forge_core::load_config(path).ok().and_then(|c| {
                c.profile.and_then(|p| p.keys().next().cloned())
            })
        })
    } else {
        None
    };

    let options = forge_core::ContextOptions {
        scopes: scope,
        excludes: exclude,
        workspace_root: current_dir.to_path_buf(),
        cache_dir,
        active_profile,
    };

    let context = ctx_engine.query(&options).await?;

    let output = match format.as_str() {
        "json" => {
            let exporter = forge_core::JsonExporter { pretty: false };
            exporter.export(&context)?
        }
        "json-pretty" | "pretty" => {
            let exporter = forge_core::JsonExporter { pretty: true };
            exporter.export(&context)?
        }
        "markdown" | "md" => {
            let exporter = forge_core::MarkdownExporter;
            exporter.export(&context)?
        }
        "mcp" => {
            let exporter = forge_core::McpExporter;
            exporter.export(&context)?
        }
        _ => {
            let exporter = forge_core::JsonExporter { pretty: false };
            exporter.export(&context)?
        }
    };

    Ok(json!({ "data": output }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_error_code() {
        assert_eq!(PARSE_ERROR, -32700);
    }

    #[test]
    fn test_invalid_request_code() {
        assert_eq!(INVALID_REQUEST, -32600);
    }

    #[test]
    fn test_method_not_found_code() {
        assert_eq!(METHOD_NOT_FOUND, -32601);
    }

    #[test]
    fn test_internal_error_code() {
        assert_eq!(INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_error_response_serialization() {
        let resp = error_response(Some(json!(1)), PARSE_ERROR, "Parse error");
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("\"code\":-32700"));
        assert!(json_str.contains("\"message\":\"Parse error\""));
        assert!(json_str.contains("\"id\":1"));
    }

    #[test]
    fn test_success_response_serialization() {
        let resp = success_response(Some(json!(1)), json!({"state": "Ready"}));
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains("\"result\""));
        assert!(json_str.contains("\"state\":\"Ready\""));
        assert!(json_str.contains("\"id\":1"));
    }

    #[test]
    fn test_notification_no_id() {
        let request: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"engine.status","params":{}}"#)
                .unwrap();
        assert!(request.id.is_none());
    }
}
