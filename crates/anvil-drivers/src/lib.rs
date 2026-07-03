use std::process::Command;

#[derive(Debug)]
pub enum PackageManager {
    Winget,
    Homebrew,
    Apt,
    Pacman,
}

pub fn detect_package_manager() -> Result<PackageManager, String> {
    let os = std::env::consts::OS;
    match os {
        "windows" => Ok(PackageManager::Winget),
        "macos" => Ok(PackageManager::Homebrew),
        "linux" => {
            // Check for apt-get
            if Command::new("apt-get").arg("--version").output().is_ok() {
                Ok(PackageManager::Apt)
            } else if Command::new("pacman").arg("--version").output().is_ok() {
                Ok(PackageManager::Pacman)
            } else {
                Err("Unsupported Linux distribution: neither apt-get nor pacman found".to_string())
            }
        }
        _ => Err(format!("Unsupported operating system: {}", os)),
    }
}

pub fn install_package(package_name: &str) -> Result<(), String> {
    let pm = detect_package_manager()?;
    let mut cmd = match pm {
        PackageManager::Winget => {
            let mut c = Command::new("winget");
            c.args(["install", "--exact", package_name]);
            c
        }
        PackageManager::Homebrew => {
            let mut c = Command::new("brew");
            c.args(["install", package_name]);
            c
        }
        PackageManager::Apt => {
            let mut c = Command::new("sudo");
            c.args(["apt-get", "install", "-y", package_name]);
            c
        }
        PackageManager::Pacman => {
            let mut c = Command::new("sudo");
            c.args(["pacman", "-S", "--noconfirm", package_name]);
            c
        }
    };

    let status = cmd.status()
        .map_err(|e| format!("Failed to spawn command {:?}: {}", cmd, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Package manager failed with exit code: {}",
            status.code().unwrap_or(-1)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_package_manager() {
        let pm = detect_package_manager();
        // Since we are running in tests, we can at least check if it does not crash
        match pm {
            Ok(p) => println!("Detected package manager: {:?}", p),
            Err(e) => println!("Package manager detection failed (might be ok on some hosts): {}", e),
        }
    }
}
