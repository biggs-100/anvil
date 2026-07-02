//! Integration tests for forge jsonrpc.
//!
//! These tests spawn the forge binary with the `jsonrpc` subcommand,
//! send JSON-RPC 2.0 requests over stdin, and verify responses from stdout.
//!
//! Run with: cargo test --test jsonrpc_test -- --nocapture
//!
//! Cargo automatically sets CARGO_BIN_EXE_FORGE_CLI when the test binary is
//! built in the same workspace as the forge-cli crate. No --ignored flag needed.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Locate the forge binary using Cargo's env var or fallback.
///
/// Cargo sets `CARGO_BIN_EXE_FORGE_CLI` (uppercase, underscore-separated)
/// when integration tests run in a workspace that produces the `forge-cli`
/// binary. The fallback is used when running outside of Cargo (e.g., IDE
/// test runner).
fn forge_exe() -> std::path::PathBuf {
    let current_exe = std::env::current_exe().expect("test binary path");
    let target_dir = current_exe.parent()
        .and_then(|p| p.parent())
        .expect("target directory");
    let exe_name = if cfg!(windows) { "forge-cli.exe" } else { "forge-cli" };
    target_dir.join(exe_name)
}

/// Helper: spawn `forge jsonrpc`, send a request, read one response line.
fn send_request(request: &str) -> String {
    let exe = forge_exe();
    let mut child = Command::new(&exe)
        .arg("jsonrpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn forge jsonrpc");

    let stdin = child.stdin.as_mut().expect("failed to get stdin");
    stdin
        .write_all(request.as_bytes())
        .expect("failed to write to stdin");
    stdin
        .write_all(b"\n")
        .expect("failed to write newline");
    stdin.flush().expect("failed to flush stdin");

    let stdout = child.stdout.as_mut().expect("failed to get stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("failed to read response");

    line.trim().to_string()
}

#[test]
fn test_engine_status_request() {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"engine.status","params":{}}"#;
    let response = send_request(request);

    assert!(!response.is_empty(), "Response should not be empty");

    // Parse as JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("Response should be valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0", "Response should use jsonrpc 2.0");
    assert_eq!(parsed["id"], 1, "Response should echo the request id");
    assert!(
        parsed["result"].is_object(),
        "Response should have a result object"
    );
    assert!(
        parsed.get("error").is_none() || parsed["error"].is_null(),
        "Response should not have an error"
    );
}

#[test]
fn test_parse_error_response() {
    let response = send_request("not valid json");

    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("Response should be valid JSON");
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(
        parsed["error"].is_object(),
        "Response should have an error object"
    );
    assert_eq!(parsed["error"]["code"], -32700, "Error code should be -32700");
}

#[test]
fn test_method_not_found() {
    let request = r#"{"jsonrpc":"2.0","id":2,"method":"nonexistent","params":{}}"#;
    let response = send_request(request);

    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("Response should be valid JSON");
    assert_eq!(parsed["id"], 2);
    assert_eq!(parsed["error"]["code"], -32601);
    assert_eq!(parsed["error"]["message"], "Method not found: nonexistent");
}

#[test]
fn test_notification_no_response() {
    // Notification requests (without id) must not produce a response
    let request = r#"{"jsonrpc":"2.0","method":"engine.status","params":{}}"#;
    let exe = forge_exe();
    let mut child = Command::new(&exe)
        .arg("jsonrpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn forge jsonrpc");

    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(request.as_bytes())
            .expect("failed to write to stdin");
        stdin
            .write_all(b"\n")
            .expect("failed to write newline");
        stdin.flush().expect("failed to flush stdin");
    }

    // Drop stdin to signal EOF
    drop(child.stdin.take());

    // Give the process time to process and potentially write a response
    std::thread::sleep(Duration::from_millis(500));

    // Read whatever is available (should be nothing for notification)
    let output = child.wait_with_output().expect("failed to wait");
    let got_response = !output.stdout.is_empty();

    assert!(
        !got_response,
        "Notification request should not produce a response, got: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_subprocess_lifecycle_error() {
    /// When the forge subprocess dies mid-request, the client should
    /// detect the connection error (EOF or broken pipe).
    let exe = forge_exe();
    let mut child = Command::new(&exe)
        .arg("jsonrpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn forge jsonrpc");

    // Send a valid request
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"engine.status","params":{}}"#;
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin
            .write_all(request.as_bytes())
            .expect("failed to write to stdin");
        stdin
            .write_all(b"\n")
            .expect("failed to write newline");
        stdin.flush().expect("failed to flush stdin");
    }

    // Read the first line to confirm the process is alive
    {
        let stdout = child.stdout.as_mut().expect("failed to get stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("failed to read response");
        let parsed: serde_json::Value =
            serde_json::from_str(line.trim()).expect("Response should be valid JSON");
        assert_eq!(parsed["jsonrpc"], "2.0");
    }

    // Kill the subprocess mid-stream
    child.kill().expect("failed to kill forge process");

    // Wait for the process to exit
    let status = child.wait().expect("failed to wait for process");
    assert!(
        !status.success(),
        "Killed process should report non-zero exit"
    );

    // Attempting to write after kill should fail (broken pipe / connection error)
    let write_result = {
        let stdin = child.stdin.as_mut();
        match stdin {
            Some(s) => s.write_all(b"more data\n"),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "stdin already dropped",
            )),
        }
    };
    assert!(
        write_result.is_err(),
        "Writing to a killed subprocess should fail with a connection error"
    );
}
