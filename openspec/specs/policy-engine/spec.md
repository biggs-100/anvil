# Policy Engine Specification

## Purpose

Provide a declarative pre-flight gate that validates anvil operations against organizational policies before execution. Policies are configured via the optional `[policy]` section in `anvil.toml` and enforced by `PolicyEngine` checks prior to each operation. Missing `[policy]` means no restrictions — full backward compatibility.

---

## Requirements

### Requirement: Policy Configuration Parsing

The system MUST parse a `[policy]` section from `anvil.toml` using the following rules and defaults:

| Rule | Type | Default | Description |
|------|------|---------|-------------|
| `allow_network` | bool | `true` | Permit network access during operations |
| `require_hashes` | bool | `false` | Require hash-verified downloads |
| `forbid_unlocked` | bool | `false` | Reject operations without a lockfile |
| `minimum_health` | u8 (0–100) | `0` | Minimum diagnostic health score |
| `required_profiles` | string[] | `[]` | Profiles that MUST be active |
| `forbid_runtimes` | string[] | `[]` | Runtimes that MUST NOT be active |

The system MUST apply all defaults when the `[policy]` section is absent or empty. The system MUST ignore unknown keys in `[policy]` without error. The system SHOULD emit a warning when a rule value is invalid (e.g., `minimum_health = 150`) and clamp it to the valid range.

#### Scenario: Full Policy Section Parsed Correctly

- GIVEN `anvil.toml` contains a `[policy]` section with `allow_network = false`, `minimum_health = 80`, and `forbid_runtimes = ["nodejs"]`
- WHEN the configuration is loaded
- THEN all 6 rules MUST be parsed with the specified values overriding defaults for `allow_network`, `minimum_health`, and `forbid_runtimes`, and defaults for unspecified rules

#### Scenario: Missing Policy Section Uses Defaults

- GIVEN `anvil.toml` exists without a `[policy]` section
- WHEN the configuration is loaded
- THEN all 6 policy rules MUST have their default values (`allow_network = true`, `minimum_health = 0`, etc.)

#### Scenario: Invalid Value Clamped with Warning

- GIVEN `anvil.toml` has `[policy]` with `minimum_health = 150`
- WHEN the configuration is loaded
- THEN the system MUST emit a warning and clamp the value to `100`

#### Scenario: Unknown Keys Ignored

- GIVEN `anvil.toml` has `[policy]` with `unknown_key = "foo"`
- WHEN the configuration is loaded
- THEN the system MUST ignore `unknown_key` without error

---

### Requirement: PolicyEngine Pre-Flight Checks

`PolicyEngine` SHALL provide three check methods that return `Result<(), PolicyViolation>`. Each violation MUST include: `rule` (rule name), `expected` (expected value), `current` (actual value), and `message` (human-readable explanation). The system MUST return ALL violations, not just the first one encountered. The system MUST NOT modify any state.

#### Scenario: All Violations Returned

- GIVEN `forbid_unlocked = true` and `minimum_health = 90` and both conditions are violated
- WHEN `check_before_up()` is called
- THEN the result MUST contain TWO violations in the error — one for each failed rule

#### Scenario: Check Passes Successfully

- GIVEN all policy rules are satisfied
- WHEN any check method is called
- THEN the result MUST be `Ok(())`

---

### Requirement: Check Before `anvil up`

`PolicyEngine::check_before_up()` MUST validate `allow_network`, `require_hashes`, and `minimum_health`.

#### Scenario: Network Policy Blocks Up

- GIVEN `allow_network = false`
- WHEN `anvil up` is invoked
- THEN the operation MUST be aborted with a violation printed to stderr

---

### Requirement: Check Before `anvil run` / `anvil shell`

`PolicyEngine::check_before_run()` MUST validate `minimum_health`, `required_profiles`, and `forbid_runtimes`.

#### Scenario: Forbidden Runtime Blocks Run

- GIVEN `forbid_runtimes = ["nodejs"]` and the active runtime is `nodejs`
- WHEN `anvil run` is invoked
- THEN the operation MUST be aborted with a violation showing the forbidden runtime name

#### Scenario: Required Profile Not Active

- GIVEN `required_profiles = ["production"]` and the active profile is `development`
- WHEN `anvil shell` is invoked
- THEN the operation MUST be aborted with a violation showing the missing profile

---

### Requirement: Check Before `anvil sync`

`PolicyEngine::check_before_sync()` MUST validate `allow_network` and `require_hashes`.

#### Scenario: Network Policy Blocks Sync

- GIVEN `allow_network = false`
- WHEN `anvil sync` is invoked
- THEN the operation MUST be aborted with a network policy violation

---

### Requirement: Missing `anvil.toml` Skips Policy Checks

If no `anvil.toml` file exists, the system MUST skip all policy checks and allow the operation to proceed.

#### Scenario: Missing anvil.toml Passes Through

- GIVEN no `anvil.toml` exists in the project directory
- WHEN any anvil command is invoked
- THEN the system MUST NOT perform any policy checks and MUST proceed with the operation

---

### Requirement: `forbid_runtimes` Ignores Unknown Runtimes

If `forbid_runtimes` lists a runtime that is not present in the project's runtime configuration, the system MUST ignore that entry (future-proofing).

#### Scenario: Unknown Runtime in forbid_runtimes

- GIVEN `forbid_runtimes = ["unknown-runtime"]` and no runtime named `unknown-runtime` is configured
- WHEN `check_before_run()` is called
- THEN the system MUST NOT flag a violation for the unknown runtime
