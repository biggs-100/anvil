# Proposal: Anvil Policy Engine

## Intent

Add a declarative security and compliance layer that gates anvil operations before execution. Currently every operation runs unconditionally — there's no mechanism to enforce organizational policies like blocking network access, requiring verified downloads, mandating a minimum health score, or restricting runtimes.

## Scope

### In Scope
- Parse `[policy]` section from `anvil.toml` (6 rules: allow_network, require_hashes, forbid_unlocked, minimum_health, required_profiles, forbid_runtimes)
- `PolicyEngine` struct with `check()` returning structured violation reports
- Pre-flight enforcement before `anvil up`, `anvil sync`, `anvil run`, `anvil shell`
- Backward compatible — missing `[policy]` means no restrictions, all existing tests pass unmodified

### Out of Scope
- Policy violations audit trail / persistent logging
- Remote policy server or centralized distribution
- Policy inheritance or composition across projects
- Hot-reloading (stateless per-operation check is sufficient)

## Capabilities

### New Capabilities
- `policy-engine`: Declarative policy engine — parses `[policy]` from anvil.toml, validates current state against 6 rules, gates operations with structured violation reports (rule name, expected, actual, explanation)

### Modified Capabilities
- None — policy is a pre-flight gate with zero changes to existing Operation trait or operation logic

## Approach

- Parse `[policy]` from anvil.toml via serde (`PolicyConfig` struct with sensible defaults)
- `PolicyEngine::new(&config)` loads rules, `engine.check(&state)` returns `Result<(), Vec<Violation>>`
- CLI command handlers call `engine.check()` before delegating to existing operation logic
- Violations include: rule name, expected value, actual value, human-readable explanation
- New crate: `crates/forge-policy-engine/`

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-config/src/` | New | `PolicyConfig` struct + serde deserialization |
| `crates/forge-policy-engine/src/lib.rs` | New | `PolicyEngine`, rule evaluation, `Violation` type |
| `crates/anvil-cli/src/commands/*.rs` | Modified | Pre-flight policy check in up/sync/run/shell handlers |
| `crates/anvil-cli/Cargo.toml` | Modified | Dependency on forge-policy-engine crate |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Policy blocks legitimate workflows | Low | Opt-in — missing `[policy]` = no restrictions. Clear error messages help users fix config. |
| `minimum_health` coupling to DiagnosticEngine | Low | Call `health_score()` — no deep coupling, just a numeric comparison. |

## Rollback Plan

Remove `[policy]` from `anvil.toml` to restore unrestricted behavior. If crate changes need reverting: `git revert` the merge commit.

## Dependencies

- `anvil.toml` parsing (existing serde/toml infrastructure)
- `DiagnosticEngine::health_score()` for `minimum_health` rule
- Profile resolution for `required_profiles` rule
- Lockfile detection (`anvil.lock` exists check) for `forbid_unlocked`

## Success Criteria

- [ ] All 6 rules parsed from `anvil.toml` with correct defaults when section is absent
- [ ] Each rule produces correct violation when its condition is triggered
- [ ] Existing operations run unchanged when `[policy]` is absent
- [ ] All existing tests pass without modification
