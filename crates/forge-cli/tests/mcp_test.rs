//! Integration tests for forge mcp.
//!
//! These tests spawn the forge binary with the `mcp` subcommand,
//! send JSON-RPC 2.0 requests over stdin, and verify responses from stdout.
//!
//! Run with: cargo test --test mcp_test -- --nocapture
//!
//! Note: Tests are #[ignore] by default because they require the forge binary
//! to be compiled first. Run with `cargo test --test mcp_test -- --ignored`
//! after a build.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Locate the forge binary using Cargo's env var or fallback.
fn forge_exe() -> String {
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_forge-cli") {
        return exe;
    }
    if let Ok(exe) = std::env::var("CARGO_BIN_EXE_forge") {
        return exe;
    }
    // Fallback: assume it's in PATH or relative path
    "forge".to_string()
}

/// Helper: send a sequence of MCP requests and collect all response lines.
///
/// Spawns `forge mcp`, writes each request line to stdin, then closes stdin
/// and reads all response lines from stdout. The responses are returned in
/// the order they were received.
fn send_mcp_requests(requests: &[&str]) -> Vec<String> {
    let exe = forge_exe();
    let mut child = Command::new(&exe)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn forge mcp");

    let stdin = child.stdin.as_mut().expect("failed to get stdin");
    for req in requests {
        stdin
            .write_all(req.as_bytes())
            .expect("failed to write to stdin");
        stdin
            .write_all(b"\n")
            .expect("failed to write newline");
    }
    stdin.flush().expect("failed to flush stdin");
    let _ = stdin; // Close stdin so child sees EOF

    let stdout = child.stdout.take().expect("failed to get stdout");
    let reader = BufReader::new(stdout);
    let mut lines = Vec::new();
    for line in reader.lines() {
        if let Ok(l) = line {
            let trimmed = l.trim().to_string();
            if !trimmed.is_empty() {
                lines.push(trimmed);
            }
        }
    }

    // Child will be reaped when it goes out of scope
    lines
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_initialize_handshake() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
    ]);

    assert_eq!(lines.len(), 1, "Should get exactly one response line");
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[0]).expect("Response should be valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert_eq!(
        parsed["result"]["protocol_version"],
        "2024-11-05"
    );
    assert!(parsed["result"]["capabilities"].is_object());
    assert!(parsed["result"]["server_info"].is_object());
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_list_tools_returns_six() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    ]);

    assert_eq!(lines.len(), 2, "Should get exactly two response lines");
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[1]).expect("Second response should be valid JSON");
    assert_eq!(parsed["id"], 2);
    let tools = parsed["result"]["tools"].as_array().expect("result.tools should be an array");
    assert_eq!(tools.len(), 6, "Should have 6 tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"forge_run"));
    assert!(names.contains(&"forge_shell"));
    assert!(names.contains(&"forge_sync"));
    assert!(names.contains(&"forge_plan"));
    assert!(names.contains(&"forge_explain"));
    assert!(names.contains(&"forge_doctor"));
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_unknown_method_returns_method_not_found() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/unknown","params":{}}"#,
    ]);

    assert!(lines.len() >= 2);
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[1]).expect("Response should be valid JSON");
    assert_eq!(parsed["error"]["code"], -32601);
    assert!(parsed["error"]["message"].as_str().unwrap_or("").contains("Method not found"));
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_list_resources_returns_forge_context() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}"#,
    ]);

    assert!(lines.len() >= 2);
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[1]).expect("Response should be valid JSON");
    let resources = parsed["result"]["resources"].as_array().expect("result.resources should be an array");
    assert!(!resources.is_empty());
    assert_eq!(resources[0]["uri"], "forge://context/active");
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_list_prompts_returns_three() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"prompts/list","params":{}}"#,
    ]);

    assert!(lines.len() >= 2);
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[1]).expect("Response should be valid JSON");
    let prompts = parsed["result"]["prompts"].as_array().expect("result.prompts should be an array");
    assert_eq!(prompts.len(), 3);
    let names: Vec<&str> = prompts.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"forge:status"));
    assert!(names.contains(&"forge:diagnose"));
    assert!(names.contains(&"forge:explain"));
}

#[test]
#[ignore = "requires compiled forge binary (cargo build)"]
fn test_read_resource_active_context() {
    let lines = send_mcp_requests(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{},"client_info":{"name":"test","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"forge://context/active"}}"#,
    ]);

    assert!(lines.len() >= 2);
    let parsed: serde_json::Value =
        serde_json::from_str(&lines[1]).expect("Response should be valid JSON");
    let contents = parsed["result"]["contents"].as_array().expect("result.contents should be an array");
    assert!(!contents.is_empty());
    assert_eq!(contents[0]["uri"], "forge://context/active");
    assert_eq!(contents[0]["mime_type"], "application/json");
}
