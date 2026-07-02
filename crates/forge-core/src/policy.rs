use serde::{Serialize, Deserialize};
use std::fmt;

fn default_true() -> bool {
    true
}

/// Policy configuration parsed from the `[policy]` section in `forge.toml`.
///
/// All fields have sensible defaults. An absent or empty `[policy]` section
/// means no restrictions — full backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Permit network access during operations (default: `true`).
    #[serde(default = "default_true")]
    pub allow_network: bool,

    /// Require hash-verified downloads (default: `false`).
    #[serde(default)]
    pub require_hashes: bool,

    /// Reject operations when no lockfile exists (default: `false`).
    #[serde(default)]
    pub forbid_unlocked: bool,

    /// Minimum diagnostic health score, 0–100 (default: `0`, clamped).
    #[serde(default)]
    pub minimum_health: u8,

    /// Profiles that MUST be active (default: empty list).
    #[serde(default)]
    pub required_profiles: Vec<String>,

    /// Runtimes that MUST NOT be active (default: empty list).
    #[serde(default)]
    pub forbid_runtimes: Vec<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            allow_network: true,
            require_hashes: false,
            forbid_unlocked: false,
            minimum_health: 0,
            required_profiles: Vec::new(),
            forbid_runtimes: Vec::new(),
        }
    }
}

/// A single policy violation: which rule was broken and what was expected vs
/// what was actually observed.
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    /// Machine-readable rule name (e.g. `"allow_network"`).
    pub rule: &'static str,
    /// The value that was expected by the policy.
    pub expected: String,
    /// The value that was actually observed.
    pub current: String,
    /// Human-readable explanation of the violation.
    pub message: String,
}

impl fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "POLICY VIOLATION: {} — {} (current: {}, expected: {})",
            self.rule, self.message, self.current, self.expected
        )
    }
}

/// Pre-flight policy engine that evaluates organisational rules against
/// current environment state.
///
/// Stateless — every check method takes the relevant state as parameters.
/// Returns _all_ violations (not just the first one).
#[derive(Debug, Clone)]
pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    /// Create a new engine from a [`PolicyConfig`].
    ///
    /// Clamps `minimum_health` to the valid 0–100 range and prints a warning
    /// to stderr when the value exceeds 100.
    pub fn new(config: &PolicyConfig) -> Self {
        let mut config = config.clone();
        if config.minimum_health > 100 {
            eprintln!(
                "Warning: minimum_health = {} exceeds maximum 100, clamping to 100",
                config.minimum_health
            );
            config.minimum_health = 100;
        }
        Self { config }
    }

    // ── Public check methods ──────────────────────────────────────────

    /// Check policy rules that apply to `forge up`:
    ///
    /// - `allow_network` — network access allowed?
    /// - `forbid_unlocked` — lockfile must exist?
    /// - `require_hashes` — lockfile entries must carry hashes?
    /// - `minimum_health` — environment health above threshold?
    pub fn check_before_up(
        &self,
        lockfile_exists: bool,
        lockfile_has_hashes: bool,
        health_score: u8,
    ) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        if !self.config.allow_network {
            violations.push(PolicyViolation {
                rule: "allow_network",
                expected: "true".into(),
                current: "false".into(),
                message: "Network access is prohibited but 'forge up' requires it".into(),
            });
        }

        if self.config.forbid_unlocked && !lockfile_exists {
            violations.push(PolicyViolation {
                rule: "forbid_unlocked",
                expected: "lockfile present".into(),
                current: "no lockfile".into(),
                message: "Unlocked operations are forbidden — no lockfile found".into(),
            });
        }

        if self.config.require_hashes && !lockfile_has_hashes {
            violations.push(PolicyViolation {
                rule: "require_hashes",
                expected: "hashes present in lockfile".into(),
                current: "lockfile without hashes".into(),
                message: "Hash verification required but lockfile entries lack hashes".into(),
            });
        }

        let threshold = self.config.minimum_health.min(100);
        if (health_score as u16) < (threshold as u16) {
            violations.push(PolicyViolation {
                rule: "minimum_health",
                expected: format!("health score >= {}", threshold),
                current: format!("health score = {}", health_score),
                message: format!(
                    "Environment health ({}) is below the minimum threshold ({})",
                    health_score, threshold,
                ),
            });
        }

        violations
    }

    /// Check policy rules that apply to `forge sync`:
    ///
    /// - `allow_network`
    /// - `require_hashes`
    pub fn check_before_sync(
        &self,
        lockfile_exists: bool,
        lockfile_has_hashes: bool,
    ) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        if !self.config.allow_network {
            violations.push(PolicyViolation {
                rule: "allow_network",
                expected: "true".into(),
                current: "false".into(),
                message: "Network access is prohibited but 'forge sync' requires it".into(),
            });
        }

        if self.config.require_hashes && !lockfile_has_hashes {
            violations.push(PolicyViolation {
                rule: "require_hashes",
                expected: "hashes present in lockfile".into(),
                current: "lockfile without hashes".into(),
                message: "Hash verification required but lockfile entries lack hashes".into(),
            });
        }

        // keep lockfile_exists for API symmetry even if not directly checked
        let _ = lockfile_exists;

        violations
    }

    /// Check policy rules that apply to `forge run` / `forge shell`:
    ///
    /// - `minimum_health`
    /// - `required_profiles`
    /// - `forbid_runtimes`
    pub fn check_before_run(
        &self,
        active_profile: Option<&str>,
        active_runtimes: &[String],
        health_score: u8,
    ) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        let threshold = self.config.minimum_health.min(100);
        if (health_score as u16) < (threshold as u16) {
            violations.push(PolicyViolation {
                rule: "minimum_health",
                expected: format!("health score >= {}", threshold),
                current: format!("health score = {}", health_score),
                message: format!(
                    "Environment health ({}) is below the minimum threshold ({})",
                    health_score, threshold,
                ),
            });
        }

        if !self.config.required_profiles.is_empty() {
            match active_profile {
                Some(profile) => {
                    if !self.config.required_profiles.iter().any(|p| p == profile) {
                        violations.push(PolicyViolation {
                            rule: "required_profiles",
                            expected: format!("one of {:?}", self.config.required_profiles),
                            current: format!("active profile: {}", profile),
                            message: format!(
                                "Active profile '{}' is not in the required list",
                                profile,
                            ),
                        });
                    }
                }
                None => {
                    violations.push(PolicyViolation {
                        rule: "required_profiles",
                        expected: format!("one of {:?}", self.config.required_profiles),
                        current: "no active profile".into(),
                        message: "Required profiles are configured but no profile is active".into(),
                    });
                }
            }
        }

        if !self.config.forbid_runtimes.is_empty() {
            for forbidden in &self.config.forbid_runtimes {
                if active_runtimes.iter().any(|rt| rt == forbidden) {
                    violations.push(PolicyViolation {
                        rule: "forbid_runtimes",
                        expected: format!("runtime '{}' not active", forbidden),
                        current: format!("runtime '{}' is active", forbidden),
                        message: format!("Runtime '{}' is forbidden but currently active", forbidden),
                    });
                }
            }
        }

        violations
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    // ── PolicyConfig serde ────────────────────────────────────────────

    #[test]
    fn test_config_defaults_when_section_absent() {
        let config: PolicyConfig = toml::from_str("").unwrap();
        assert!(config.allow_network, "allow_network defaults to true");
        assert!(!config.require_hashes, "require_hashes defaults to false");
        assert!(!config.forbid_unlocked, "forbid_unlocked defaults to false");
        assert_eq!(config.minimum_health, 0, "minimum_health defaults to 0");
        assert!(config.required_profiles.is_empty(), "required_profiles defaults to empty");
        assert!(config.forbid_runtimes.is_empty(), "forbid_runtimes defaults to empty");
    }

    #[test]
    fn test_config_parses_from_toml_full_section() {
        let toml_str = r#"
allow_network = false
require_hashes = true
forbid_unlocked = true
minimum_health = 80
required_profiles = ["production"]
forbid_runtimes = ["nodejs"]
"#;
        let config: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.allow_network);
        assert!(config.require_hashes);
        assert!(config.forbid_unlocked);
        assert_eq!(config.minimum_health, 80);
        assert_eq!(config.required_profiles, vec!["production"]);
        assert_eq!(config.forbid_runtimes, vec!["nodejs"]);
    }

    #[test]
    fn test_unknown_keys_ignored() {
        let toml_str = r#"
unknown_key = "foo"
allow_network = false
"#;
        let config: PolicyConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.allow_network);
    }

    // ── minimum_health clamping ───────────────────────────────────────

    #[test]
    fn test_minimum_health_clamped_to_100_with_warning() {
        let raw = PolicyConfig {
            minimum_health: 150,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        // Clamped to 100, so health=100 should pass
        let violations = engine.check_before_up(true, true, 100);
        assert!(
            violations.is_empty(),
            "Expected no violations when health=100 after clamp from 150"
        );
    }

    #[test]
    fn test_minimum_health_in_range_not_clamped() {
        let raw = PolicyConfig {
            minimum_health: 50,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(true, true, 49);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "minimum_health");
    }

    // ── check_before_up ───────────────────────────────────────────────

    #[test]
    fn test_before_up_all_rules_pass() {
        let engine = PolicyEngine::new(&PolicyConfig::default());
        let violations = engine.check_before_up(true, true, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_up_allow_network_false() {
        let raw = PolicyConfig {
            allow_network: false,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(true, true, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "allow_network");
    }

    #[test]
    fn test_before_up_forbid_unlocked_no_lockfile() {
        let raw = PolicyConfig {
            forbid_unlocked: true,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(false, false, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "forbid_unlocked");
    }

    #[test]
    fn test_before_up_forbid_unlocked_with_lockfile_passes() {
        let raw = PolicyConfig {
            forbid_unlocked: true,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(true, false, 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_up_require_hashes_no_hashes() {
        let raw = PolicyConfig {
            require_hashes: true,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(true, false, 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "require_hashes");
    }

    #[test]
    fn test_before_up_all_violations_returned() {
        // forbid_unlocked + minimum_health + allow_network + require_hashes all fail
        let raw = PolicyConfig {
            allow_network: false,
            require_hashes: true,
            forbid_unlocked: true,
            minimum_health: 90,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_up(false, false, 50);
        assert_eq!(violations.len(), 4);
    }

    // ── check_before_sync ─────────────────────────────────────────────

    #[test]
    fn test_before_sync_all_rules_pass() {
        let engine = PolicyEngine::new(&PolicyConfig::default());
        let violations = engine.check_before_sync(true, true);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_sync_allow_network_false() {
        let raw = PolicyConfig {
            allow_network: false,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_sync(true, true);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "allow_network");
    }

    #[test]
    fn test_before_sync_require_hashes_no_hashes() {
        let raw = PolicyConfig {
            require_hashes: true,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_sync(true, false);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "require_hashes");
    }

    // ── check_before_run ──────────────────────────────────────────────

    #[test]
    fn test_before_run_all_rules_pass() {
        let engine = PolicyEngine::new(&PolicyConfig::default());
        let violations = engine.check_before_run(Some("dev"), &[], 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_run_forbid_runtimes_blocks_specific() {
        let raw = PolicyConfig {
            forbid_runtimes: vec!["nodejs".into()],
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(Some("dev"), &["nodejs".into()], 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "forbid_runtimes");
    }

    #[test]
    fn test_before_run_forbid_runtimes_ignores_unknown() {
        let raw = PolicyConfig {
            forbid_runtimes: vec!["unknown-runtime".into()],
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(Some("dev"), &["nodejs".into()], 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_run_required_profiles_mismatch() {
        let raw = PolicyConfig {
            required_profiles: vec!["production".into()],
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(Some("development"), &[], 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "required_profiles");
    }

    #[test]
    fn test_before_run_required_profiles_match() {
        let raw = PolicyConfig {
            required_profiles: vec!["production".into()],
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(Some("production"), &[], 100);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_before_run_required_profiles_no_active() {
        let raw = PolicyConfig {
            required_profiles: vec!["production".into()],
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(None, &[], 100);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "required_profiles");
    }

    #[test]
    fn test_before_run_low_health() {
        let raw = PolicyConfig {
            minimum_health: 80,
            ..Default::default()
        };
        let engine = PolicyEngine::new(&raw);
        let violations = engine.check_before_run(Some("dev"), &[], 50);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "minimum_health");
    }

    // ── Display ───────────────────────────────────────────────────────

    #[test]
    fn test_policy_violation_display_format() {
        let v = PolicyViolation {
            rule: "allow_network",
            expected: "true".into(),
            current: "false".into(),
            message: "Network access is prohibited".into(),
        };
        let output = v.to_string();
        assert!(output.contains("POLICY VIOLATION:"));
        assert!(output.contains("allow_network"));
        assert!(output.contains("Network access is prohibited"));
        assert!(output.contains("current: false"));
        assert!(output.contains("expected: true"));
    }
}
