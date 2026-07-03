use std::process::Command;

#[test]
fn test_cli_context_help() {
    let bin_path = env!("CARGO_BIN_EXE_anvil");
    let output = Command::new(bin_path)
        .arg("context")
        .arg("--help")
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Display active environment configuration") || stdout.contains("context"));
}

#[test]
fn test_cli_context_json() {
    let bin_path = env!("CARGO_BIN_EXE_anvil");
    let output = Command::new(bin_path)
        .arg("context")
        .arg("--format")
        .arg("json")
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));
    
    // Parse as JSON to check valid structure
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.get("schema_version").unwrap().as_str().unwrap(), "1.0.0");
}

#[test]
fn test_cli_context_markdown() {
    let bin_path = env!("CARGO_BIN_EXE_anvil");
    let output = Command::new(bin_path)
        .arg("context")
        .arg("--format")
        .arg("markdown")
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Command failed: {:?}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("# Anvil Context Summary"));
}
