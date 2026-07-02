use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeVersion(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmulationLog {
    pub requested: String,
    pub installed: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeLock {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emulation: Option<EmulationLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Lockfile {
    #[serde(rename = "runtime", default)]
    pub runtimes: Vec<RuntimeLock>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleState {
    Uninitialized,
    Initialized,
    Resolved,
    Locked,
    Synced,
    Ready,
    Active,
    Dirty,
    Outdated,
    Broken,
}

impl LifecycleState {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Uninitialized, Self::Initialized) => true,
            (Self::Initialized, Self::Resolved) => true,
            (Self::Resolved, Self::Locked) => true,
            (Self::Locked, Self::Synced) => true,
            (Self::Synced, Self::Ready) => true,
            (Self::Ready, Self::Active) => true,
            (Self::Active, Self::Ready) => true,
            (Self::Locked | Self::Synced | Self::Ready | Self::Active, Self::Dirty) => true,
            (Self::Locked | Self::Synced | Self::Ready | Self::Active, Self::Outdated) => true,
            (Self::Locked | Self::Synced | Self::Ready | Self::Active | Self::Dirty, Self::Broken) => true,
            (Self::Dirty | Self::Broken, Self::Resolved | Self::Locked) => true,
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OperationStatus {
    Success,
    Failure,
    Warning,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeRecord {
    pub path: String,
    pub action: String, // "added" | "modified" | "deleted"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResult {
    pub status: OperationStatus,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
    pub changes: Vec<ChangeRecord>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventStatus {
    Started,
    Progress(u8),
    Finished,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub timestamp: String,
    pub operation_id: String,
    pub runtime: String,
    pub phase: String,
    pub status: EventStatus,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_transitions() {
        // Test transitioning from LOCKED to READY to ACTIVE
        assert!(LifecycleState::Locked.can_transition_to(&LifecycleState::Synced));
        assert!(LifecycleState::Synced.can_transition_to(&LifecycleState::Ready));
        assert!(LifecycleState::Ready.can_transition_to(&LifecycleState::Active));
        assert!(LifecycleState::Active.can_transition_to(&LifecycleState::Ready));
        
        // Invalid transitions
        assert!(!LifecycleState::Uninitialized.can_transition_to(&LifecycleState::Ready));
        assert!(!LifecycleState::Locked.can_transition_to(&LifecycleState::Active));
        
        // Dirty / Broken recovery transitions
        assert!(LifecycleState::Ready.can_transition_to(&LifecycleState::Dirty));
        assert!(LifecycleState::Dirty.can_transition_to(&LifecycleState::Broken));
        assert!(LifecycleState::Broken.can_transition_to(&LifecycleState::Locked));
        assert!(LifecycleState::Dirty.can_transition_to(&LifecycleState::Resolved));
    }

    #[test]
    fn test_lockfile_serialization_with_emulation() {
        let lockfile = Lockfile {
            runtimes: vec![
                RuntimeLock {
                    name: "node".to_string(),
                    version: "20.10.0".to_string(),
                    platform: "windows".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://example.com/node.zip".to_string(),
                    size: 1234,
                    sha256: "hash123".to_string(),
                    emulation: Some(EmulationLog {
                        requested: "windows-arm64".to_string(),
                        installed: "windows-x86_64".to_string(),
                        reason: "Native build unavailable".to_string(),
                    }),
                },
                RuntimeLock {
                    name: "python".to_string(),
                    version: "3.11.0".to_string(),
                    platform: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    url: "https://example.com/python.tar.gz".to_string(),
                    size: 5678,
                    sha256: "hash456".to_string(),
                    emulation: None,
                },
            ],
        };

        let serialized = toml::to_string_pretty(&lockfile).unwrap();
        assert!(serialized.contains("emulation"));
        assert!(serialized.contains("requested = \"windows-arm64\""));
        assert!(serialized.contains("installed = \"windows-x86_64\""));
        assert!(serialized.contains("reason = \"Native build unavailable\""));

        let deserialized: Lockfile = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.runtimes.len(), 2);
        
        let node_lock = deserialized.runtimes.iter().find(|r| r.name == "node").unwrap();
        assert_eq!(node_lock.emulation, Some(EmulationLog {
            requested: "windows-arm64".to_string(),
            installed: "windows-x86_64".to_string(),
            reason: "Native build unavailable".to_string(),
        }));

        let python_lock = deserialized.runtimes.iter().find(|r| r.name == "python").unwrap();
        assert_eq!(python_lock.emulation, None);
    }
}
