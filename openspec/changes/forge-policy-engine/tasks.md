# Tasks: Forge Policy Engine

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~310 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: Foundation — Policy Module

- [x] 1.1 Create `crates/forge-core/src/policy.rs` — `PolicyConfig` struct with 6 fields + serde defaults (`#[serde(default = "default_true")]` for `allow_network`)
- [x] 1.2 Add `PolicyViolation` struct (`rule`, `expected`, `current`, `message`) with `Display` impl printing violation line to stderr
- [x] 1.3 Add `PolicyEngine` struct with `new(&PolicyConfig)` and helper for clamping `minimum_health` (0..=100) with `eprintln!` warning
- [x] 1.4 Implement `check_before_up()` — validates `allow_network`, `forbid_unlocked`, `require_hashes`, `minimum_health`
- [x] 1.5 Implement `check_before_sync()` — validates `allow_network`, `require_hashes`
- [x] 1.6 Implement `check_before_run()` — validates `minimum_health`, `required_profiles`, `forbid_runtimes`

## Phase 2: Integration — Manifest Wiring

- [x] 2.1 In `crates/forge-core/src/manifest.rs`, add `#[serde(default)] pub policy: Option<PolicyConfig>` to `ForgeConfig`
- [x] 2.2 In `crates/forge-core/src/lib.rs`, add `pub mod policy;` and `pub use policy::{PolicyConfig, PolicyEngine, PolicyViolation};`

## Phase 3: CLI Enforcement

- [x] 3.1 In `crates/forge-cli/src/main.rs`, add helper `build_policy_engine()` to build `PolicyEngine` from forge.toml (returns `None` if no `[policy]` section or no forge.toml found)
- [x] 3.2 Add policy check before `Commands::Up` — call `check_before_up()`; print violations to stderr via `enforce_policy()` and `process::exit(1)` on hits
- [x] 3.3 Add policy check before `Commands::Sync` — call `check_before_sync()` with same violation-exit pattern
- [x] 3.4 Add policy check before `Commands::Run` — call `check_before_run()` passing active_profile, active_runtimes, health_score
- [x] 3.5 Add policy check before `Commands::Shell` — call `check_before_run()` same as Run

## Phase 4: Testing

- [x] 4.1 Unit test: `PolicyConfig` parsed from TOML with all 6 rules — assert values match
- [x] 4.2 Unit test: `PolicyConfig` defaults when `[policy]` section absent
- [x] 4.3 Unit test: `minimum_health` clamped to 100 with warning on `minimum_health = 150`
- [x] 4.4 Unit test: `check_before_up` — `allow_network = false` returns violation; all-pass returns empty vec
- [x] 4.5 Unit test: `check_before_sync` — `require_hashes = true` with hashless lockfile returns violation
- [x] 4.6 Unit test: `check_before_run` — `forbid_runtimes` matches active runtime; `required_profiles` mismatch
- [x] 4.7 Unit test: `forbid_runtimes` ignores unknown runtimes (not configured — no violation)
- [x] 4.8 Unit test: `PolicyViolation::Display` output includes rule name, expected, current, message
- [x] 4.9 Integration: CLI helper returns `None` when no forge.toml exists (skips checks)
