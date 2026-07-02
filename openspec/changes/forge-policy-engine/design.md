# Design: Forge Policy Engine

## Technical Approach

Add a `policy` module inside `forge-core` that parses an optional `[policy]` section from `forge.toml` and gates operations via pre-flight checks. The `PolicyEngine` is a stateless evaluator — it takes config + snapshot of current state, returns all violations at once. CLI handlers call it before Up/Sync/Run/Shell, passing the required context (lockfile state, health score, active profile, active runtimes). No changes to operations themselves.

## Architecture Decisions

| Decision | Alternatives | Choice & Rationale |
|----------|-------------|--------------------|
| Module location | Separate crate `forge-policy-engine` | **Module in forge-core** (`policy.rs`). Same pattern as `diagnostics`, `operations`. No new dependency graph edge. Direct access to manifest types. |
| PolicyConfig placement | Inline fields in ForgeConfig | **Separate `PolicyConfig` struct** ref'd as `policy: Option<PolicyConfig>`. Follows existing `ConfigSection` / `ProfileSection` pattern. Clean `#[serde(default)]` on the Option. |
| Check method shape | Single `check(op_kind)` with flags | **Three methods** — `check_before_up`, `check_before_sync`, `check_before_run`. Each takes only the state it needs. Clearer at call sites and self-documenting. |
| Violation return | Return first violation only | **Return `Vec<PolicyViolation>`**. Spec requires ALL violations. Caller iterates and prints all before aborting. |
| `minimum_health` coupling | PolicyEngine calls DiagnosticEngine directly | **PolicyEngine accepts `health_score: u8`**. Caller evaluates diagnostic health and passes the score. Keeps PolicyEngine stateless and testable without DiagnosticEngine dependency. |
| Violation display | JSON or structured output | **`Display` impl on `PolicyViolation`**. Simple human-readable lines printed to stderr. Fits existing CLI style (see `eprintln!` patterns in main.rs). |

## Data Flow

```
forge.toml ──serde──→ ForgeConfig
                          │
                    policy: Option<PolicyConfig>
                          │
                          ▼
                   PolicyEngine::new(config)
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
   check_before_up  check_before_sync  check_before_run
          │               │               │
          └─────── Vec<PolicyViolation> ──┘
                          │
                     [empty] → proceed
                     [hits]  → print violations → exit(1)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/policy.rs` | Create | `PolicyConfig`, `PolicyEngine`, `PolicyViolation`, check methods |
| `crates/forge-core/src/manifest.rs` | Modify | Add `policy: Option<PolicyConfig>` to `ForgeConfig` |
| `crates/forge-core/src/lib.rs` | Modify | Re-export `pub mod policy` and key types |
| `crates/forge-cli/src/main.rs` | Modify | Add policy checks before `Up`, `Sync`, `Run`, `Shell` arms |

## Interfaces / Contracts

```rust
// policy.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(default = "default_true")]
    pub allow_network: bool,
    #[serde(default)]
    pub require_hashes: bool,
    #[serde(default)]
    pub forbid_unlocked: bool,
    #[serde(default)]
    pub minimum_health: u8,         // clamped 0..=100
    #[serde(default)]
    pub required_profiles: Vec<String>,
    #[serde(default)]
    pub forbid_runtimes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub rule: &'static str,
    pub expected: String,
    pub current: String,
    pub message: String,
}

pub struct PolicyEngine { config: PolicyConfig }

impl PolicyEngine {
    pub fn new(config: &PolicyConfig) -> Self;

    // validates: allow_network, require_hashes, minimum_health
    pub fn check_before_up(
        &self, lockfile_exists: bool,
        lockfile_has_hashes: bool, health_score: u8,
    ) -> Vec<PolicyViolation>;

    // validates: allow_network, require_hashes
    pub fn check_before_sync(
        &self, lockfile_exists: bool,
        lockfile_has_hashes: bool,
    ) -> Vec<PolicyViolation>;

    // validates: minimum_health, required_profiles, forbid_runtimes
    pub fn check_before_run(
        &self, active_profile: Option<&str>,
        active_runtimes: &[String], health_score: u8,
    ) -> Vec<PolicyViolation>;
}
```

### manifest.rs changes

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    pub runtimes: HashMap<String, String>,
    pub workspace_id: Option<String>,
    pub config: Option<ConfigSection>,
    pub profile: Option<HashMap<String, ProfileSection>>,
    #[serde(default)]                       // ← new field
    pub policy: Option<PolicyConfig>,        // ← new field
}
```

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | Each rule in isolation (policy_config defaults, clamping, violation triggers) | Pure function tests — construct `PolicyEngine` with known config, call check, assert violations |
| Unit | `PolicyConfig` serde (absent section, full section, unknown keys, invalid value) | Round-trip through `toml::from_str` / `toml::to_string` |
| Integration | `PolicyEngine` wired in CLI dispatch | Test that missing forge.toml skips checks; test that violations print and abort |
| Edge | `minimum_health` clamping (0, 100, 150) | Assert value is clamped to valid range with warning |

## Migration / Rollout

No migration required. `[policy]` is optional — absent section = zero restrictions, all existing tests pass unmodified.

## Open Questions

- None resolved. All design decisions are covered by the spec and codebase analysis.
