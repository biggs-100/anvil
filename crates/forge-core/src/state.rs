// TODO: tests — depends on Engine lifecycle
use std::path::Path;
use crate::types::LifecycleState;

/// Compute the current lifecycle state from workspace artifacts.
pub fn compute_current_state(workspace_root: &Path, cache_dir: &Path) -> LifecycleState {
    let toml_path = workspace_root.join("forge.toml");
    if !toml_path.exists() {
        return LifecycleState::Uninitialized;
    }
    
    let config = match crate::manifest::load_config(&toml_path) {
        Ok(c) => c,
        Err(_) => return LifecycleState::Broken,
    };
    
    let lock_path = workspace_root.join("forge.lock");
    if !lock_path.exists() {
        return LifecycleState::Initialized;
    }
    
    let lockfile = match crate::load_lockfile(&lock_path) {
        Ok(l) => l,
        Err(_) => return LifecycleState::Broken,
    };

    let mut config_runtimes = config.runtimes.clone();
    for runtime in &lockfile.runtimes {
        if config_runtimes.remove(&runtime.name).is_none() {
            return LifecycleState::Outdated;
        }
    }
    if !config_runtimes.is_empty() {
        return LifecycleState::Outdated;
    }

    let mut all_healthy = true;
    let mut any_missing = false;
    for runtime in &lockfile.runtimes {
        let target_dir = cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
        if !target_dir.exists() {
            any_missing = true;
            all_healthy = false;
        } else {
            if let Ok(mut entries) = std::fs::read_dir(&target_dir) {
                if entries.next().is_none() {
                    all_healthy = false;
                }
            } else {
                all_healthy = false;
            }
        }
    }

    if any_missing {
        return LifecycleState::Locked;
    }
    if !all_healthy {
        return LifecycleState::Broken;
    }

    let shims_cache = workspace_root.join(".forge").join("shims.cache");
    if !shims_cache.exists() {
        return LifecycleState::Synced;
    }

    let state_json = workspace_root.join(".forge").join("state.json");
    if state_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&state_json) {
            if content.contains("\"Dirty\"") {
                return LifecycleState::Dirty;
            }
        }
    }

    LifecycleState::Ready
}

/// Persist the current lifecycle state to `.forge/state.json`.
pub fn save_state(workspace_root: &Path, state: LifecycleState) {
    let state_path = workspace_root.join(".forge").join("state.json");
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&state_path, serde_json::to_string(&state).unwrap_or_default());
}
