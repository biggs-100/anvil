use std::path::PathBuf;
use std::collections::HashMap;
use std::process::Command;

pub fn run_command_in_env(
    cmd: &str,
    args: &[String],
    env_vars: &HashMap<String, String>,
    bin_dirs: &[PathBuf],
) -> Result<i32, String> {
    let mut command = Command::new(cmd);
    command.args(args);
    
    for (k, v) in env_vars {
        command.env(k, v);
    }
    
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path_str = String::new();
    
    #[cfg(windows)]
    let sep = ";";
    #[cfg(not(windows))]
    let sep = ":";
    
    for p in bin_dirs {
        if !path_str.is_empty() {
            path_str.push_str(sep);
        }
        path_str.push_str(&p.to_string_lossy());
    }
    
    if !current_path.is_empty() {
        if !path_str.is_empty() {
            path_str.push_str(sep);
        }
        path_str.push_str(&current_path.to_string_lossy());
    }
    
    command.env("PATH", path_str);
    
    let mut child = command.spawn()
        .map_err(|e| format!("Failed to spawn command '{}': {}", cmd, e))?;
        
    let status = child.wait()
        .map_err(|e| format!("Failed to wait for command '{}': {}", cmd, e))?;
        
    Ok(status.code().unwrap_or(0))
}

pub fn spawn_shell_in_env(
    env_vars: &HashMap<String, String>,
    bin_dirs: &[PathBuf],
) -> Result<i32, String> {
    #[cfg(windows)]
    let (shell_cmd, shell_args) = if let Ok(comspec) = std::env::var("COMSPEC") {
        (comspec, vec![])
    } else {
        ("powershell.exe".to_string(), vec![])
    };
    
    #[cfg(not(windows))]
    let (shell_cmd, shell_args) = if let Ok(shell) = std::env::var("SHELL") {
        (shell, vec![])
    } else {
        ("/bin/sh".to_string(), vec![])
    };
    
    run_command_in_env(&shell_cmd, &shell_args, env_vars, bin_dirs)
}
