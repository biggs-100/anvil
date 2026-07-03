# Environment Lifecycle RFC Specification

## Purpose

Define the formal RFC-0011 specification for the environment lifecycle, comprising 10 distinct states, state invariants, and transition rules.

## Requirements

### Requirement: State Definitions and Invariants
The system MUST support and track exactly 10 distinct environment states:
1. **UNINITIALIZED**: No project configuration or metadata exists.
2. **INITIALIZED**: Project configuration (e.g. `anvil.toml`) exists.
3. **RESOLVED**: Target toolchain versions mapped and resolved against remote providers.
4. **LOCKED**: Lockfile (`anvil.lock`) exists with resolved versions and SHA-256 hashes.
5. **SYNCED**: All locked toolchains downloaded, verified, and staged in `.anvil/staging`.
6. **READY**: Staged runtimes committed to `.anvil/runtimes` and `.anvil/shims.cache` regenerated.
7. **ACTIVE**: Current shell/session paths prepended with the active environment's shims.
8. **DIRTY**: Local runtimes or shims altered or mutated outside the system control.
9. **OUTDATED**: Configuration or lockfile updated, mismatching the current READY runtimes.
10. **BROKEN**: Critical file corruption, missing binaries, or failed transition states.

### Requirement: Lifecycle Transitions
The system MUST transition between states only via valid CLI operations or direct environment validations.

| Source State | Target State | Triggering Operation / Event |
|---|---|---|
| UNINITIALIZED | INITIALIZED | `init` command |
| INITIALIZED | RESOLVED | `resolve` command |
| RESOLVED | LOCKED | `lock` command |
| LOCKED | SYNCED | `sync` command (download and stage) |
| SYNCED | READY | Staging promotion/commit phase |
| READY | ACTIVE | `shell` or `run` command execution |
| READY / ACTIVE | DIRTY | Manual file mutation or validation mismatch |
| READY / ACTIVE | OUTDATED | Configuration change / Lockfile mismatch |
| Any State | BROKEN | Unrecoverable error / Validation crash |
| BROKEN | READY | `repair` command success |

#### Scenario: Successful Init Transition
- GIVEN the project directory is UNINITIALIZED
- WHEN the `init` command is executed
- THEN the system MUST transition to the INITIALIZED state and write a new configuration file.

#### Scenario: Detection of Outdated State
- GIVEN the system is in a READY state
- WHEN the `anvil.toml` configuration is modified to request a different Python version
- THEN the system MUST transition to the OUTDATED state on the next check.

#### Scenario: Recovery from Broken State
- GIVEN the environment is in a BROKEN state due to missing runtime shims
- WHEN the `repair` command is successfully executed
- THEN the system MUST restore the missing files and transition back to the READY state.
