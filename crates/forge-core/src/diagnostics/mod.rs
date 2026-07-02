use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::watch;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticMode {
    Fast,
    Deep,
}

pub struct DiagnosticContext {
    pub workspace_root: PathBuf,
    pub cache_dir: PathBuf,
    pub mode: DiagnosticMode,
    pub active_profile: Option<String>,
}

impl crate::environment::RuntimeContextProvider for DiagnosticContext {
    fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
    fn runtime_path(&self, name: &str) -> Option<PathBuf> {
        let lockfile_path = self.workspace_root.join("forge.lock");
        if lockfile_path.exists() {
            if let Ok(lockfile) = crate::lock::load_lockfile(&lockfile_path) {
                if let Some(r) = lockfile.runtimes.iter().find(|rt| rt.name == name) {
                    return Some(self.cache_dir.join(&r.name).join(&r.version).join("extracted"));
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    INFO,
    WARNING,
    ERROR,
    CRITICAL,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Explanation {
    pub what: String,
    pub why: String,
    pub how: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickFix {
    pub description: String,
    pub action: QuickFixAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum QuickFixAction {
    WipeAndReextract { runtime_name: String, version: String },
    RecreateShim { shim_name: String },
    SetEnvVar { key: String, value: String },
    SetSecret { key: String },
    RegenerateLockfile,
    RegenerateShimsCache,
    AddToGitIgnore { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Finding {
    pub code: String,
    pub category: String,
    pub severity: Severity,
    pub confidence: u8,
    pub message: String,
    pub explanation: Explanation,
    pub suggested_quick_fix: Option<QuickFix>,
    pub doc_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticReport {
    pub timestamp: String,
    pub mode: DiagnosticMode,
    pub health_score: u8,
    pub findings: Vec<Finding>,
    pub elapsed_ms: u64,
}

pub fn mask_sensitive_text(text: &str) -> String {
    let mut masked = text.to_string();
    for (key, val) in std::env::vars() {
        if crate::environment::is_secret(&key) && !val.is_empty() && val.len() > 3 {
            masked = masked.replace(&val, "[MASKED]");
        }
    }
    masked
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum QuickFixActionShadow<'a> {
    WipeAndReextract { runtime_name: &'a str, version: &'a str },
    RecreateShim { shim_name: &'a str },
    SetEnvVar { key: &'a str, value: &'a str },
    SetSecret { key: &'a str },
    RegenerateLockfile,
    RegenerateShimsCache,
    AddToGitIgnore { path: &'a str },
}

impl serde::Serialize for QuickFixAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let shadow = match self {
            Self::WipeAndReextract { runtime_name, version } => {
                QuickFixActionShadow::WipeAndReextract { runtime_name, version }
            }
            Self::RecreateShim { shim_name } => {
                QuickFixActionShadow::RecreateShim { shim_name }
            }
            Self::SetEnvVar { key, value } => {
                let val = if crate::environment::is_secret(key) {
                    "[MASKED]"
                } else {
                    value.as_str()
                };
                QuickFixActionShadow::SetEnvVar { key, value: val }
            }
            Self::SetSecret { key } => {
                QuickFixActionShadow::SetSecret { key }
            }
            Self::RegenerateLockfile => QuickFixActionShadow::RegenerateLockfile,
            Self::RegenerateShimsCache => QuickFixActionShadow::RegenerateShimsCache,
            Self::AddToGitIgnore { path } => QuickFixActionShadow::AddToGitIgnore { path },
        };
        shadow.serialize(serializer)
    }
}

impl serde::Serialize for Finding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let masked_message = mask_sensitive_text(&self.message);
        let masked_explanation = Explanation {
            what: mask_sensitive_text(&self.explanation.what),
            why: mask_sensitive_text(&self.explanation.why),
            how: mask_sensitive_text(&self.explanation.how),
        };

        #[derive(Serialize)]
        struct FindingShadow<'a> {
            code: &'a str,
            category: &'a str,
            severity: Severity,
            confidence: u8,
            message: &'a str,
            explanation: &'a Explanation,
            suggested_quick_fix: &'a Option<QuickFix>,
            doc_url: &'a Option<String>,
        }

        let shadow = FindingShadow {
            code: &self.code,
            category: &self.category,
            severity: self.severity,
            confidence: self.confidence,
            message: &masked_message,
            explanation: &masked_explanation,
            suggested_quick_fix: &self.suggested_quick_fix,
            doc_url: &self.doc_url,
        };
        shadow.serialize(serializer)
    }
}

pub fn calculate_health_score(findings: &[Finding]) -> u8 {
    let mut score: i32 = 100;
    let mut has_critical = false;
    for finding in findings {
        match finding.severity {
            Severity::CRITICAL => {
                score -= 30;
                has_critical = true;
            }
            Severity::ERROR => {
                score -= 15;
            }
            Severity::WARNING => {
                score -= 5;
            }
            Severity::INFO => {}
        }
    }
    let mut final_score = score.clamp(0, 100) as u8;
    if has_critical && final_score > 40 {
        final_score = 40;
    }
    final_score
}

pub trait HealthCheck: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn category(&self) -> &'static str;
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>>;
}

#[derive(Clone, Debug)]
pub enum NodeStatus {
    Pending,
    Running,
    Success(Vec<Finding>),
    Failed(String),
    Skipped { blocker: String },
}

pub struct DiagnosticEngine {
    pub checks: Vec<Arc<dyn HealthCheck>>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            checks: vec![
                Arc::new(ManifestCheck),
                Arc::new(LockCheck),
                Arc::new(RuntimeCheck),
                Arc::new(HashCheck),
                Arc::new(SecretCheck),
                Arc::new(EnvironmentCheck),
                Arc::new(PathCheck),
                Arc::new(ShimCheck),
                Arc::new(CacheCheck),
                Arc::new(ProviderCheck),
                Arc::new(ProfileCheck),
            ],
        }
    }

    pub fn with_checks(checks: Vec<Arc<dyn HealthCheck>>) -> Self {
        Self { checks }
    }

    /// Registers plugin health checks alongside built-in checks.
    /// Fast/Deep mode filtering applies automatically via `DiagnosticContext.mode`.
    pub fn register_plugin_checks(&mut self, checks: Vec<Arc<dyn HealthCheck>>) {
        self.checks.extend(checks);
    }

    pub async fn run(&self, ctx: &DiagnosticContext) -> DiagnosticReport {
        let start_time = std::time::Instant::now();
        
        let mut watch_senders = HashMap::new();
        let mut watch_receivers = HashMap::new();
        
        for check in &self.checks {
            let (tx, rx) = watch::channel(NodeStatus::Pending);
            watch_senders.insert(check.id(), tx);
            watch_receivers.insert(check.id(), rx);
        }
        
        let ctx = Arc::new(DiagnosticContext {
            workspace_root: ctx.workspace_root.clone(),
            cache_dir: ctx.cache_dir.clone(),
            mode: ctx.mode,
            active_profile: ctx.active_profile.clone(),
        });
        
        let mut join_handles = Vec::new();
        
        for check in &self.checks {
            let check = check.clone();
            let tx = watch_senders.get(check.id()).unwrap().clone();
            
            let mut dep_rxs = Vec::new();
            for dep_id in check.dependencies() {
                if let Some(rx) = watch_receivers.get(dep_id) {
                    dep_rxs.push((dep_id, rx.clone()));
                }
            }
            
            let ctx_clone = ctx.clone();
            
            let handle = tokio::spawn(async move {
                let mut skipped_because_of = None;
                
                for (dep_id, mut rx) in dep_rxs {
                    loop {
                        let status = rx.borrow().clone();
                        match status {
                            NodeStatus::Pending | NodeStatus::Running => {}
                            NodeStatus::Failed(_) => {
                                skipped_because_of = Some(dep_id.to_string());
                                break;
                            }
                            NodeStatus::Skipped { .. } => {
                                skipped_because_of = Some(dep_id.to_string());
                                break;
                            }
                            NodeStatus::Success(ref findings) => {
                                let blocked = findings.iter().any(|f| {
                                    f.severity == Severity::CRITICAL || f.severity == Severity::ERROR
                                });
                                if blocked {
                                    skipped_because_of = Some(dep_id.to_string());
                                }
                                break;
                            }
                        }
                        if rx.changed().await.is_err() {
                            break;
                        }
                    }
                    if skipped_because_of.is_some() {
                        break;
                    }
                }
                
                if let Some(blocker) = skipped_because_of {
                    let _ = tx.send(NodeStatus::Skipped { blocker });
                    return;
                }
                
                let _ = tx.send(NodeStatus::Running);
                match check.check(&ctx_clone).await {
                    Ok(findings) => {
                        let _ = tx.send(NodeStatus::Success(findings));
                    }
                    Err(err) => {
                        let _ = tx.send(NodeStatus::Failed(err));
                    }
                }
            });
            join_handles.push(handle);
        }
        
        for handle in join_handles {
            let _ = handle.await;
        }
        
        let mut all_findings = Vec::new();
        
        for check in &self.checks {
            let rx = watch_receivers.get(check.id()).unwrap();
            let status = rx.borrow().clone();
            match status {
                NodeStatus::Success(findings) => {
                    all_findings.extend(findings);
                }
                NodeStatus::Failed(err) => {
                    all_findings.push(Finding {
                        code: check.id().to_string(),
                        category: check.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Check '{}' execution failed: {}", check.name(), err),
                        explanation: Explanation {
                            what: "Health check execution error".to_string(),
                            why: "An internal error occurred during verification".to_string(),
                            how: "Check the check details and environment".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    });
                }
                NodeStatus::Skipped { blocker } => {
                    all_findings.push(Finding {
                        code: check.id().to_string(),
                        category: check.category().to_string(),
                        severity: Severity::WARNING,
                        confidence: 100,
                        message: format!(
                            "Check '{}' skipped because upstream dependency '{}' failed or was skipped",
                            check.name(),
                            blocker
                        ),
                        explanation: Explanation {
                            what: "Check skipped".to_string(),
                            why: format!("This check depends on '{}' which failed to execute successfully", blocker),
                            how: "Resolve the upstream critical blocker first".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    });
                }
                _ => {}
            }
        }
        
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let health_score = calculate_health_score(&all_findings);
        let timestamp = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("{}", now)
        };
        
        DiagnosticReport {
            timestamp,
            mode: ctx.mode,
            health_score,
            findings: all_findings,
            elapsed_ms,
        }
    }
}

// 1. ManifestCheck
pub struct ManifestCheck;
impl HealthCheck for ManifestCheck {
    fn id(&self) -> &'static str { "FG001" }
    fn name(&self) -> &'static str { "ManifestCheck" }
    fn description(&self) -> &'static str { "Validates existence and syntax of forge.toml" }
    fn category(&self) -> &'static str { "manifest" }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let toml_path = ctx.workspace_root.join("forge.toml");
            if !toml_path.exists() {
                return Ok(vec![Finding {
                    code: "FG001".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::CRITICAL,
                    confidence: 100,
                    message: "Missing forge.toml manifest file".to_string(),
                    explanation: Explanation {
                        what: "The project manifest file (forge.toml) is missing".to_string(),
                        why: "Forge requires a forge.toml file in the workspace root to define managed runtimes".to_string(),
                        how: "Run 'forge init' to initialize a new forge.toml manifest".to_string(),
                    },
                    suggested_quick_fix: Some(QuickFix {
                        description: "Initialize forge.toml".to_string(),
                        action: QuickFixAction::RegenerateLockfile,
                    }),
                    doc_url: Some("https://forge.sh/docs/manifest".to_string()),
                }]);
            }
            match crate::manifest::load_config(&toml_path) {
                Ok(_) => Ok(Vec::new()),
                Err(err) => Ok(vec![Finding {
                    code: "FG002".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::ERROR,
                    confidence: 100,
                    message: format!("Syntax/deserialization errors in forge.toml: {}", err),
                    explanation: Explanation {
                        what: "Failed to parse the forge.toml configuration".to_string(),
                        why: "The manifest file contains invalid TOML syntax or structural errors".to_string(),
                        how: "Fix the syntax errors in forge.toml".to_string(),
                    },
                    suggested_quick_fix: None,
                    doc_url: Some("https://forge.sh/docs/manifest".to_string()),
                }]),
            }
        }.boxed()
    }
}

// 2. LockCheck
pub struct LockCheck;
impl HealthCheck for LockCheck {
    fn id(&self) -> &'static str { "FG003" }
    fn name(&self) -> &'static str { "LockCheck" }
    fn description(&self) -> &'static str { "Validates existence and sync status of forge.lock" }
    fn category(&self) -> &'static str { "lock" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG001"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let lock_path = ctx.workspace_root.join("forge.lock");
            if !lock_path.exists() {
                return Ok(vec![Finding {
                    code: "FG003".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::ERROR,
                    confidence: 100,
                    message: "Missing forge.lock lockfile".to_string(),
                    explanation: Explanation {
                        what: "The dependency lockfile (forge.lock) is missing".to_string(),
                        why: "Forge uses the lockfile to ensure reproducible runtime installations".to_string(),
                        how: "Run 'forge lock' or 'forge install' to generate forge.lock".to_string(),
                    },
                    suggested_quick_fix: Some(QuickFix {
                        description: "Regenerate the lockfile".to_string(),
                        action: QuickFixAction::RegenerateLockfile,
                    }),
                    doc_url: Some("https://forge.sh/docs/lockfile".to_string()),
                }]);
            }
            let toml_path = ctx.workspace_root.join("forge.toml");
            let config = match crate::manifest::load_config(&toml_path) {
                Ok(cfg) => cfg,
                Err(_) => return Ok(Vec::new()),
            };
            let lockfile = match crate::lock::load_lockfile(&lock_path) {
                Ok(lf) => lf,
                Err(err) => {
                    return Ok(vec![Finding {
                        code: "FG003".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Failed to parse forge.lock: {}", err),
                        explanation: Explanation {
                            what: "The lockfile is corrupted".to_string(),
                            why: "The file forge.lock contains invalid structure".to_string(),
                            how: "Regenerate the lockfile".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: "Regenerate the lockfile".to_string(),
                            action: QuickFixAction::RegenerateLockfile,
                        }),
                        doc_url: None,
                    }]);
                }
            };
            let mut out_of_sync = false;
            for (name, version_req) in &config.runtimes {
                if let Some(lock) = lockfile.runtimes.iter().find(|r| &r.name == name) {
                    if let Ok(req) = semver::VersionReq::parse(version_req.version()) {
                        if let Ok(ver) = semver::Version::parse(&lock.version) {
                            if !req.matches(&ver) {
                                out_of_sync = true;
                            }
                        } else {
                            out_of_sync = true;
                        }
                    } else {
                        out_of_sync = true;
                    }
                } else {
                    out_of_sync = true;
                }
            }
            if out_of_sync {
                Ok(vec![Finding {
                    code: "FG004".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::WARNING,
                    confidence: 100,
                    message: "Manifest and lockfile dependencies are out of sync".to_string(),
                    explanation: Explanation {
                        what: "Dependencies defined in forge.toml do not match forge.lock".to_string(),
                        why: "A runtime was added, removed, or changed in forge.toml without updating the lockfile".to_string(),
                        how: "Run 'forge install' or 'forge lock' to synchronize dependencies".to_string(),
                    },
                    suggested_quick_fix: Some(QuickFix {
                        description: "Regenerate the lockfile".to_string(),
                        action: QuickFixAction::RegenerateLockfile,
                    }),
                    doc_url: Some("https://forge.sh/docs/lockfile".to_string()),
                }])
            } else {
                Ok(Vec::new())
            }
        }.boxed()
    }
}

// 3. RuntimeCheck
pub struct RuntimeCheck;
impl HealthCheck for RuntimeCheck {
    fn id(&self) -> &'static str { "FG005" }
    fn name(&self) -> &'static str { "RuntimeCheck" }
    fn description(&self) -> &'static str { "Validates extraction and execution of configured runtimes" }
    fn category(&self) -> &'static str { "runtime" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG003"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let lock_path = ctx.workspace_root.join("forge.lock");
            let lockfile = match crate::lock::load_lockfile(&lock_path) {
                Ok(lf) => lf,
                Err(_) => return Ok(Vec::new()),
            };
            let mut findings = Vec::new();
            for runtime in &lockfile.runtimes {
                let target_extract_to = ctx.cache_dir.join(&runtime.name).join(&runtime.version).join("extracted");
                let mut folder_healthy = false;
                if target_extract_to.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&target_extract_to) {
                        if entries.next().is_some() {
                            folder_healthy = true;
                        }
                    }
                }
                if !folder_healthy {
                    findings.push(Finding {
                        code: "FG005".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Target extraction folder for runtime {}-{} is missing or empty", runtime.name, runtime.version),
                        explanation: Explanation {
                            what: "The runtime extraction folder is missing or incomplete".to_string(),
                            why: "The runtime installation might have been interrupted or deleted from cache".to_string(),
                            how: "Run 'forge install' to re-extract the runtime".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: format!("Wipe and re-extract {}", runtime.name),
                            action: QuickFixAction::WipeAndReextract {
                                runtime_name: runtime.name.clone(),
                                version: runtime.version.clone(),
                            },
                        }),
                        doc_url: None,
                    });
                    continue;
                }
                if ctx.mode == DiagnosticMode::Deep {
                    let bin_dirs = crate::cache::find_bin_dirs(&target_extract_to);
                    let mut found_executable = None;
                    for bin_dir in &bin_dirs {
                        if let Ok(entries) = std::fs::read_dir(bin_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_file() {
                                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                        let name_lower = filename.to_lowercase();
                                        if name_lower == runtime.name.to_lowercase()
                                            || name_lower == format!("{}.exe", runtime.name.to_lowercase())
                                            || (runtime.name == "rust" && (name_lower == "rustc" || name_lower == "rustc.exe"))
                                        {
                                            found_executable = Some(path.clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if found_executable.is_some() {
                            break;
                        }
                    }
                    if let Some(exe_path) = found_executable {
                        let arg = match runtime.name.as_str() {
                            "node" => "--version",
                            "python" | "python3" => "--version",
                            "bun" => "--version",
                            "go" => "version",
                            "cargo" => "--version",
                            "rust" | "rustc" => "--version",
                            _ => "--version",
                        };
                        let mut cmd = std::process::Command::new(&exe_path);
                        cmd.arg(arg);
                        match cmd.output() {
                            Ok(output) if output.status.success() => {}
                            Ok(output) => {
                                findings.push(Finding {
                                    code: "FG006".to_string(),
                                    category: self.category().to_string(),
                                    severity: Severity::CRITICAL,
                                    confidence: 100,
                                    message: format!(
                                        "Runtime binary execution fails or crashes on test command. Exit code: {:?}",
                                        output.status.code()
                                    ),
                                    explanation: Explanation {
                                        what: "The runtime binary executed but returned an error status".to_string(),
                                        why: "The binary might be corrupted, incompatible with current OS, or missing dynamic links".to_string(),
                                        how: "Wipe and reinstall the runtime".to_string(),
                                    },
                                    suggested_quick_fix: Some(QuickFix {
                                        description: format!("Wipe and re-extract {}", runtime.name),
                                        action: QuickFixAction::WipeAndReextract {
                                            runtime_name: runtime.name.clone(),
                                            version: runtime.version.clone(),
                                        },
                                    }),
                                    doc_url: None,
                                });
                            }
                            Err(err) => {
                                findings.push(Finding {
                                    code: "FG006".to_string(),
                                    category: self.category().to_string(),
                                    severity: Severity::CRITICAL,
                                    confidence: 100,
                                    message: format!("Failed to spawn test execution for runtime {}: {}", runtime.name, err),
                                    explanation: Explanation {
                                        what: "The system was unable to run the runtime executable".to_string(),
                                        why: "The binary permissions might be wrong, or the binary is not executable".to_string(),
                                        how: "Re-extract the runtime to restore standard permissions and executable layout".to_string(),
                                    },
                                    suggested_quick_fix: Some(QuickFix {
                                        description: format!("Wipe and re-extract {}", runtime.name),
                                        action: QuickFixAction::WipeAndReextract {
                                            runtime_name: runtime.name.clone(),
                                            version: runtime.version.clone(),
                                        },
                                    }),
                                    doc_url: None,
                                });
                            }
                        }
                    } else {
                        findings.push(Finding {
                            code: "FG005".to_string(),
                            category: self.category().to_string(),
                            severity: Severity::ERROR,
                            confidence: 100,
                            message: format!("Could not find the executable binary for runtime {}", runtime.name),
                            explanation: Explanation {
                                what: "Binary executable missing in extracted directory".to_string(),
                                why: "The extraction might have skipped the main binary, or the directory structure is unexpected".to_string(),
                                how: "Re-extract the runtime".to_string(),
                            },
                            suggested_quick_fix: Some(QuickFix {
                                description: format!("Wipe and re-extract {}", runtime.name),
                                action: QuickFixAction::WipeAndReextract {
                                    runtime_name: runtime.name.clone(),
                                    version: runtime.version.clone(),
                                },
                            }),
                            doc_url: None,
                        });
                    }
                }
            }
            Ok(findings)
        }.boxed()
    }
}

// Helper SHA-256 calculation
fn compute_sha256(path: &Path) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 64];
    loop {
        let n = file.read(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

// 4. HashCheck
pub struct HashCheck;
impl HealthCheck for HashCheck {
    fn id(&self) -> &'static str { "FG007" }
    fn name(&self) -> &'static str { "HashCheck" }
    fn description(&self) -> &'static str { "Verifies the cryptographic integrity of downloaded runtime packages" }
    fn category(&self) -> &'static str { "hash" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG005"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            if ctx.mode == DiagnosticMode::Fast {
                return Ok(Vec::new());
            }
            let lock_path = ctx.workspace_root.join("forge.lock");
            let lockfile = match crate::lock::load_lockfile(&lock_path) {
                Ok(lf) => lf,
                Err(_) => return Ok(Vec::new()),
            };
            let mut findings = Vec::new();
            for runtime in &lockfile.runtimes {
                let dest_dir = ctx.cache_dir.join(&runtime.name).join(&runtime.version);
                let filename = runtime.url.split('/').last().unwrap_or("archive");
                let dest_path = dest_dir.join(filename);
                
                if !dest_path.exists() {
                    findings.push(Finding {
                        code: "FG007".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::CRITICAL,
                        confidence: 100,
                        message: format!("Archive file missing for runtime {}-{}, cannot verify SHA-256", runtime.name, runtime.version),
                        explanation: Explanation {
                            what: "Downloaded archive file is missing from cache".to_string(),
                            why: "The archive was deleted or never saved properly, preventing cryptographic validation".to_string(),
                            how: "Trigger a fresh installation to re-download the archive".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: format!("Wipe and re-extract {}", runtime.name),
                            action: QuickFixAction::WipeAndReextract {
                                runtime_name: runtime.name.clone(),
                                version: runtime.version.clone(),
                            },
                        }),
                        doc_url: None,
                    });
                    continue;
                }
                match compute_sha256(&dest_path) {
                    Ok(computed_hash) => {
                        if computed_hash != runtime.sha256 {
                            findings.push(Finding {
                                code: "FG007".to_string(),
                                category: self.category().to_string(),
                                severity: Severity::CRITICAL,
                                confidence: 100,
                                message: format!(
                                    "Extracted runtime files fail SHA-256 verification for {}. Expected: {}, computed: {}",
                                    runtime.name, runtime.sha256, computed_hash
                                ),
                                explanation: Explanation {
                                    what: "Cryptographic hash mismatch detected".to_string(),
                                    why: "The downloaded archive file is corrupted, modified, or truncated".to_string(),
                                    how: "Delete the archive and re-download by running forge install".to_string(),
                                },
                                suggested_quick_fix: Some(QuickFix {
                                    description: format!("Wipe and re-extract {}", runtime.name),
                                    action: QuickFixAction::WipeAndReextract {
                                        runtime_name: runtime.name.clone(),
                                        version: runtime.version.clone(),
                                    },
                                }),
                                doc_url: None,
                            });
                        }
                    }
                    Err(err) => {
                        findings.push(Finding {
                            code: "FG007".to_string(),
                            category: self.category().to_string(),
                            severity: Severity::CRITICAL,
                            confidence: 100,
                            message: format!("Failed to compute SHA-256 for runtime {}: {}", runtime.name, err),
                            explanation: Explanation {
                                what: "Error reading archive file for hashing".to_string(),
                                why: "An I/O error occurred during file hashing".to_string(),
                                how: "Re-extract the runtime".to_string(),
                            },
                            suggested_quick_fix: Some(QuickFix {
                                description: format!("Wipe and re-extract {}", runtime.name),
                                action: QuickFixAction::WipeAndReextract {
                                    runtime_name: runtime.name.clone(),
                                    version: runtime.version.clone(),
                                },
                            }),
                            doc_url: None,
                        });
                    }
                }
            }
            Ok(findings)
        }.boxed()
    }
}

// 5. SecretCheck
pub struct SecretCheck;
impl HealthCheck for SecretCheck {
    fn id(&self) -> &'static str { "FG008" }
    fn name(&self) -> &'static str { "SecretCheck" }
    fn description(&self) -> &'static str { "Validates secrets store availability and format integrity" }
    fn category(&self) -> &'static str { "secrets" }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let mut findings = Vec::new();
            let secrets_path = ctx.workspace_root.join(".forge").join("secrets.json");
            if secrets_path.exists() {
                let content = match std::fs::read_to_string(&secrets_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return Ok(vec![Finding {
                            code: "FG008".to_string(),
                            category: self.category().to_string(),
                            severity: Severity::ERROR,
                            confidence: 100,
                            message: format!("Failed to read secrets.json: {}", e),
                            explanation: Explanation {
                                what: "Secrets file read failure".to_string(),
                                why: "System lacks read permissions or file is locked".to_string(),
                                how: "Fix file permissions".to_string(),
                            },
                            suggested_quick_fix: None,
                            doc_url: None,
                        }]);
                    }
                };
                let payload: Result<crate::secrets::EncryptedPayload, _> = serde_json::from_str(&content);
                if payload.is_err() {
                    findings.push(Finding {
                        code: "FG008".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: "Credentials format invalid: secrets.json is not a valid encrypted payload".to_string(),
                        explanation: Explanation {
                            what: "Invalid encrypted secrets structure".to_string(),
                            why: "The secrets.json file structure is corrupt or matches an outdated format".to_string(),
                            how: "Re-encrypt or delete secrets.json and re-configure".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: "Add secrets.json to .gitignore to prevent leakage".to_string(),
                            action: QuickFixAction::AddToGitIgnore { path: ".forge/secrets.json".to_string() },
                        }),
                        doc_url: None,
                    });
                } else if std::env::var("FORGE_MASTER_KEY").is_err() {
                    findings.push(Finding {
                        code: "FG008".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: "FORGE_MASTER_KEY master password env variable is not configured".to_string(),
                        explanation: Explanation {
                            what: "Master key is missing".to_string(),
                            why: "Decryption of secrets.json requires the master passphrase in FORGE_MASTER_KEY".to_string(),
                            how: "Define FORGE_MASTER_KEY in your system environment".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: "Set secret FORGE_MASTER_KEY".to_string(),
                            action: QuickFixAction::SetSecret { key: "FORGE_MASTER_KEY".to_string() },
                        }),
                        doc_url: None,
                    });
                }
            }
            Ok(findings)
        }.boxed()
    }
}

// 6. EnvironmentCheck
pub struct EnvironmentCheck;
impl HealthCheck for EnvironmentCheck {
    fn id(&self) -> &'static str { "FG009" }
    fn name(&self) -> &'static str { "EnvironmentCheck" }
    fn description(&self) -> &'static str { "Resolves and validates environment configurations" }
    fn category(&self) -> &'static str { "env" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG001"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let toml_path = ctx.workspace_root.join("forge.toml");
            let config = match crate::manifest::load_config(&toml_path) {
                Ok(cfg) => cfg,
                Err(_) => return Ok(Vec::new()),
            };
            let mut findings = Vec::new();
            match crate::environment::materialize_environment(ctx, &HashMap::new(), ctx.active_profile.as_deref()) {
                Ok(resolved) => {
                    if let Some(config_sec) = config.config {
                        if let Ok(issues) = crate::resolver::validate_environment(&resolved.vars, &config_sec.definitions) {
                            for issue in issues {
                                let key = issue.id.split("::").last().unwrap_or("").to_string();
                                findings.push(Finding {
                                    code: "FG009".to_string(),
                                    category: self.category().to_string(),
                                    severity: Severity::ERROR,
                                    confidence: 100,
                                    message: issue.message,
                                    explanation: Explanation {
                                        what: "Mandatory environment variable missing or invalid".to_string(),
                                        why: "The configuration manifest requires this variable to be defined and correctly typed".to_string(),
                                        how: issue.remediation,
                                    },
                                    suggested_quick_fix: Some(QuickFix {
                                        description: format!("Set environment variable {}", key),
                                        action: QuickFixAction::SetEnvVar { key, value: "".to_string() },
                                    }),
                                    doc_url: None,
                                });
                            }
                        }
                    }
                }
                Err(err) => {
                    findings.push(Finding {
                        code: "FG009".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Environment activation failed: {}", err),
                        explanation: Explanation {
                            what: "Failed to resolve environment state".to_string(),
                            why: "One or more variables are defined incorrectly or references could not be interpolated".to_string(),
                            how: "Verify forge.toml profiles and variables".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    });
                }
            }
            Ok(findings)
        }.boxed()
    }
}

// 7. PathCheck
pub struct PathCheck;
impl HealthCheck for PathCheck {
    fn id(&self) -> &'static str { "FG010" }
    fn name(&self) -> &'static str { "PathCheck" }
    fn description(&self) -> &'static str { "Checks if the global Forge shim folder is in the system PATH" }
    fn category(&self) -> &'static str { "path" }
    fn check<'a>(&'a self, _ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let home = match dirs::home_dir() {
                Some(h) => h,
                None => return Ok(Vec::new()),
            };
            let forge_bin = home.join(".forge").join("bin");
            let path_var = std::env::var_os("PATH").unwrap_or_default();
            let paths: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
            let in_path = paths.iter().any(|p| {
                if let (Ok(p1), Ok(p2)) = (p.canonicalize(), forge_bin.canonicalize()) {
                    p1 == p2
                } else {
                    p == &forge_bin
                }
            });
            if !in_path {
                Ok(vec![Finding {
                    code: "FG010".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::WARNING,
                    confidence: 100,
                    message: format!("Shim directory ({:?}) is not present in the system's PATH", forge_bin),
                    explanation: Explanation {
                        what: "Forge shim folder is missing from system PATH".to_string(),
                        why: "To run runtimes natively from terminal, your shell must look inside ~/.forge/bin".to_string(),
                        how: "Append ~/.forge/bin to your PATH environment variable in your shell profile".to_string(),
                    },
                    suggested_quick_fix: None,
                    doc_url: Some("https://forge.sh/docs/installation#path".to_string()),
                }])
            } else {
                Ok(Vec::new())
            }
        }.boxed()
    }
}

// 8. ShimCheck
pub struct ShimCheck;
impl HealthCheck for ShimCheck {
    fn id(&self) -> &'static str { "FG011" }
    fn name(&self) -> &'static str { "ShimCheck" }
    fn description(&self) -> &'static str { "Verifies integrity of shims cache map against lockfile runtimes" }
    fn category(&self) -> &'static str { "shim" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG003"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let lock_path = ctx.workspace_root.join("forge.lock");
            let lockfile = match crate::lock::load_lockfile(&lock_path) {
                Ok(lf) => lf,
                Err(_) => return Ok(Vec::new()),
            };
            let expected_map = crate::cache::generate_shims_cache_map(&lockfile, &ctx.cache_dir);
            let cache_file_path = ctx.workspace_root.join(".forge").join("shims.cache");
            if !cache_file_path.exists() {
                return Ok(vec![Finding {
                    code: "FG011".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::ERROR,
                    confidence: 100,
                    message: "shims.cache file is missing".to_string(),
                    explanation: Explanation {
                        what: "Shim mapping cache file does not exist".to_string(),
                        why: "Shims cache has not been compiled or was deleted".to_string(),
                        how: "Run 'forge shims setup' or trigger repair".to_string(),
                    },
                    suggested_quick_fix: Some(QuickFix {
                        description: "Regenerate shims cache".to_string(),
                        action: QuickFixAction::RegenerateShimsCache,
                    }),
                    doc_url: None,
                }]);
            }
            let content = match std::fs::read_to_string(&cache_file_path) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(vec![Finding {
                        code: "FG011".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Failed to read shims.cache: {}", e),
                        explanation: Explanation {
                            what: "Shims cache read failure".to_string(),
                            why: "System cannot access shims.cache".to_string(),
                            how: "Regenerate shims cache".to_string(),
                        },
                        suggested_quick_fix: Some(QuickFix {
                            description: "Regenerate shims cache".to_string(),
                            action: QuickFixAction::RegenerateShimsCache,
                        }),
                        doc_url: None,
                    }]);
                }
            };
            let mut actual_map = HashMap::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(pos) = trimmed.find('=') {
                    let key = trimmed[..pos].trim().to_string();
                    let val = trimmed[pos + 1..].trim().to_string();
                    actual_map.insert(key, PathBuf::from(val));
                }
            }
            let mut discrepancy = false;
            if expected_map.len() != actual_map.len() {
                discrepancy = true;
            } else {
                for (key, path) in &expected_map {
                    match actual_map.get(key) {
                        Some(actual_path) => {
                            let norm_path = path.to_string_lossy().replace('\\', "/");
                            let norm_actual = actual_path.to_string_lossy().replace('\\', "/");
                            if norm_path != norm_actual {
                                discrepancy = true;
                                break;
                            }
                        }
                        None => {
                            discrepancy = true;
                            break;
                        }
                    }
                }
            }
            if discrepancy {
                Ok(vec![Finding {
                    code: "FG011".to_string(),
                    category: self.category().to_string(),
                    severity: Severity::ERROR,
                    confidence: 100,
                    message: "Discrepancy between lockfile shims and compiled shims cache".to_string(),
                    explanation: Explanation {
                        what: "Active shims cache does not match the lockfile runtimes".to_string(),
                        why: "The shims cache was not updated after lockfile changes".to_string(),
                        how: "Regenerate shims cache to realign with lockfile".to_string(),
                    },
                    suggested_quick_fix: Some(QuickFix {
                        description: "Regenerate shims cache".to_string(),
                        action: QuickFixAction::RegenerateShimsCache,
                    }),
                    doc_url: None,
                }])
            } else {
                Ok(Vec::new())
            }
        }.boxed()
    }
}

// 9. CacheCheck
pub struct CacheCheck;
impl HealthCheck for CacheCheck {
    fn id(&self) -> &'static str { "FG012" }
    fn name(&self) -> &'static str { "CacheCheck" }
    fn description(&self) -> &'static str { "Measures cache disk consumption" }
    fn category(&self) -> &'static str { "cache" }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let mut total_size = 0u64;
            let mut file_count = 0u32;
            fn walk_dir(dir: &Path, size: &mut u64, count: &mut u32) {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Ok(meta) = path.metadata() {
                                *size += meta.len();
                                *count += 1;
                            }
                        } else if path.is_dir() {
                            walk_dir(&path, size, count);
                        }
                    }
                }
            }
            if ctx.cache_dir.exists() {
                walk_dir(&ctx.cache_dir, &mut total_size, &mut file_count);
            }
            let size_mb = total_size as f64 / 1024.0 / 1024.0;
            Ok(vec![Finding {
                code: "FG012".to_string(),
                category: self.category().to_string(),
                severity: Severity::INFO,
                confidence: 100,
                message: format!("Cache disk usage is {:.2} MB ({} files)", size_mb, file_count),
                explanation: Explanation {
                    what: "Reports active cache folder size".to_string(),
                    why: "Allows monitoring disk usage to prevent exhausting storage".to_string(),
                    how: "To free space, run 'forge cache clean'".to_string(),
                },
                suggested_quick_fix: None,
                doc_url: None,
            }])
        }.boxed()
    }
}

// 10. ProviderCheck
pub struct ProviderCheck;
impl HealthCheck for ProviderCheck {
    fn id(&self) -> &'static str { "FG013" }
    fn name(&self) -> &'static str { "ProviderCheck" }
    fn description(&self) -> &'static str { "Pings toolchain registries to confirm network reachability" }
    fn category(&self) -> &'static str { "provider" }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            if ctx.mode == DiagnosticMode::Fast {
                return Ok(Vec::new());
            }
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .map_err(|e| format!("Failed to build client: {}", e))?;
            match client.get("https://nodejs.org/dist/index.json").send().await {
                Ok(resp) if resp.status().is_success() => {
                    Ok(vec![Finding {
                        code: "FG013".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::INFO,
                        confidence: 100,
                        message: "Remote toolchain registries are reachable".to_string(),
                        explanation: Explanation {
                            what: "Verified internet connectivity to registry.nodejs.org".to_string(),
                            why: "Network connection is required to download new runtimes".to_string(),
                            how: "No action required".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    }])
                }
                _ => {
                    Ok(vec![Finding {
                        code: "FG013".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::WARNING,
                        confidence: 80,
                        message: "Remote registry check failed. Installation of new runtimes might fail.".to_string(),
                        explanation: Explanation {
                            what: "Remote registries are offline or unreachable".to_string(),
                            why: "Your machine is disconnected, or behind a restrictive firewall/proxy".to_string(),
                            how: "Check your internet connection or proxy settings".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    }])
                }
            }
        }.boxed()
    }
}

// 11. ProfileCheck
pub struct ProfileCheck;
impl HealthCheck for ProfileCheck {
    fn id(&self) -> &'static str { "FG014" }
    fn name(&self) -> &'static str { "ProfileCheck" }
    fn description(&self) -> &'static str { "Validates active environment profile configuration inside forge.toml" }
    fn category(&self) -> &'static str { "profile" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["FG001"] }
    fn check<'a>(&'a self, ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
        async move {
            let toml_path = ctx.workspace_root.join("forge.toml");
            let config = match crate::manifest::load_config(&toml_path) {
                Ok(cfg) => cfg,
                Err(_) => return Ok(Vec::new()),
            };
            let mut findings = Vec::new();
            if let Some(ref active_profile) = ctx.active_profile {
                let mut profile_exists = false;
                if let Some(ref profiles) = config.profile {
                    if profiles.contains_key(active_profile) {
                        profile_exists = true;
                    }
                }
                if !profile_exists {
                    findings.push(Finding {
                        code: "FG014".to_string(),
                        category: self.category().to_string(),
                        severity: Severity::ERROR,
                        confidence: 100,
                        message: format!("Active profile '{}' is not defined in forge.toml", active_profile),
                        explanation: Explanation {
                            what: "Active environment profile is missing".to_string(),
                            why: "The CLI is set to run under profile but it doesn't exist in manifest profiles list".to_string(),
                            how: "Define the profile section in forge.toml or change the active profile".to_string(),
                        },
                        suggested_quick_fix: None,
                        doc_url: None,
                    });
                }
            }
            Ok(findings)
        }.boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculations() {
        let f_info = Finding {
            code: "I".to_string(), category: "c".to_string(), severity: Severity::INFO, confidence: 100,
            message: "msg".to_string(), explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
            suggested_quick_fix: None, doc_url: None
        };
        let f_warn = Finding {
            code: "W".to_string(), category: "c".to_string(), severity: Severity::WARNING, confidence: 100,
            message: "msg".to_string(), explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
            suggested_quick_fix: None, doc_url: None
        };
        let f_err = Finding {
            code: "E".to_string(), category: "c".to_string(), severity: Severity::ERROR, confidence: 100,
            message: "msg".to_string(), explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
            suggested_quick_fix: None, doc_url: None
        };
        let f_crit = Finding {
            code: "C".to_string(), category: "c".to_string(), severity: Severity::CRITICAL, confidence: 100,
            message: "msg".to_string(), explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
            suggested_quick_fix: None, doc_url: None
        };

        assert_eq!(calculate_health_score(&[f_info.clone()]), 100);
        assert_eq!(calculate_health_score(&[f_warn.clone()]), 95);
        assert_eq!(calculate_health_score(&[f_err.clone(), f_warn.clone(), f_warn.clone()]), 75);
        assert_eq!(calculate_health_score(&[f_crit.clone(), f_warn.clone()]), 40);

        let findings = vec![f_err.clone(), f_err.clone(), f_err.clone(), f_err.clone(), f_err.clone(), f_err.clone(), f_err.clone()];
        assert_eq!(calculate_health_score(&findings), 0);
    }



    struct MockCriticalCheck;
    impl HealthCheck for MockCriticalCheck {
        fn id(&self) -> &'static str { "MOCK_CRIT" }
        fn name(&self) -> &'static str { "MockCriticalCheck" }
        fn description(&self) -> &'static str { "description" }
        fn category(&self) -> &'static str { "category" }
        fn check<'a>(&'a self, _ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
            async move {
                Ok(vec![Finding {
                    code: "FG001".to_string(), category: "c".to_string(), severity: Severity::CRITICAL, confidence: 100,
                    message: "critical failure".to_string(),
                    explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
                    suggested_quick_fix: None, doc_url: None
                }])
            }.boxed()
        }
    }

    struct MockDependentCheck;
    impl HealthCheck for MockDependentCheck {
        fn id(&self) -> &'static str { "MOCK_DEP" }
        fn name(&self) -> &'static str { "MockDependentCheck" }
        fn description(&self) -> &'static str { "description" }
        fn category(&self) -> &'static str { "category" }
        fn dependencies(&self) -> Vec<&'static str> { vec!["MOCK_CRIT"] }
        fn check<'a>(&'a self, _ctx: &'a DiagnosticContext) -> BoxFuture<'a, Result<Vec<Finding>, String>> {
            async move {
                Ok(vec![Finding {
                    code: "DEP_ERR".to_string(), category: "c".to_string(), severity: Severity::WARNING, confidence: 100,
                    message: "should not run".to_string(),
                    explanation: Explanation { what: "wt".to_string(), why: "wy".to_string(), how: "hw".to_string() },
                    suggested_quick_fix: None, doc_url: None
                }])
            }.boxed()
        }
    }

    #[tokio::test]
    async fn test_dag_scheduler_short_circuit() {
        let temp_dir = std::env::temp_dir().join("forge_diag_test_dag");
        let _ = std::fs::create_dir_all(&temp_dir);
        let ctx = DiagnosticContext {
            workspace_root: temp_dir.clone(),
            cache_dir: temp_dir.clone(),
            mode: DiagnosticMode::Fast,
            active_profile: None,
        };

        let engine = DiagnosticEngine::with_checks(vec![
            Arc::new(MockCriticalCheck),
            Arc::new(MockDependentCheck),
        ]);

        let report = engine.run(&ctx).await;
        
        let dep_finding = report.findings.iter().find(|f| f.code == "MOCK_DEP");
        assert!(dep_finding.is_some(), "Should record a skipped trace finding");
        let dep_finding = dep_finding.unwrap();
        assert_eq!(dep_finding.severity, Severity::WARNING);
        assert!(dep_finding.message.contains("skipped because upstream dependency"));
        
        let run_finding = report.findings.iter().find(|f| f.code == "DEP_ERR");
        assert!(run_finding.is_none(), "Dependent check should not have run");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
