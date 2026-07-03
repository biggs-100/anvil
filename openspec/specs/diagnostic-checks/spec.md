# Diagnostic Checks Specification

## Purpose
Define the verification logic, severities, and error codes (FG001 to FG011) for all 11 health checks, distinguishing between Fast and Deep execution modes.

## Requirements

### Requirement: Execution Mode Differentiation
The diagnostic system MUST support Fast and Deep modes to optimize execution speed.

| Mode | Allowed Operations | Skipped Operations |
|---|---|---|
| **Fast** | File existence, basic parsing, env presence checks | Hash calculations, remote pings, subprocess executions |
| **Deep** | All operations (including cryptography, network, process launch) | None |

#### Scenario: Running HashCheck in Fast Mode
- GIVEN a diagnostic context with `DiagnosticMode::Fast`
- WHEN `HashCheck` is executed
- THEN the check MUST immediately return success with zero findings

#### Scenario: Running HashCheck in Deep Mode
- GIVEN a diagnostic context with `DiagnosticMode::Deep`
- WHEN `HashCheck` is executed
- THEN the system MUST compute and verify SHA-256 checksums of extracted runtimes

---

### Requirement: 11 Diagnostic Checks Matrix
The system MUST implement 11 checks matching the following codes, categories, severities, and logic:

| Code | Check Name | Category | Severity | Detection Logic |
|---|---|---|---|---|
| **FG001** | ManifestCheck | manifest | CRITICAL | Missing `anvil.toml` manifest file |
| **FG002** | ManifestCheck | manifest | ERROR | Syntax/deserialization errors in `anvil.toml` |
| **FG003** | LockCheck | lock | ERROR | Missing `anvil.lock` lockfile |
| **FG004** | LockCheck | lock | WARNING | Manifest and lockfile dependencies are out of sync |
| **FG005** | RuntimeCheck | runtime | ERROR | Target extraction folder for runtime is missing or empty |
| **FG006** | RuntimeCheck | runtime | CRITICAL | Runtime binary execution fails or crashes on test command |
| **FG007** | HashCheck | hash | CRITICAL | Extracted runtime files fail SHA-256 verification |
| **FG008** | SecretCheck | secrets | ERROR | Credentials format invalid or remote key provider handshake fails |
| **FG009** | EnvironmentCheck| env | ERROR | Mandatory environment variables missing in active profile |
| **FG010** | PathCheck | path | WARNING | Shim directory is not present in the system's `$PATH` |
| **FG011** | ShimCheck | shim | ERROR | Discrepancy between lockfile shims and compiled shims cache |

#### Scenario: Runtime Binary Execution Failure in Deep Mode
- GIVEN a corrupted runtime binary in deep mode
- WHEN `RuntimeCheck` runs a test command on the binary
- THEN the check MUST return a `Finding` with code `FG006` and severity `CRITICAL`

---

### Requirement: Plugin-Registered Health Checks

The `DiagnosticEngine` MUST accept `HealthCheck` implementations registered via `PluginRegistry`. Plugin health checks MUST implement the same check interface as built-in checks and MUST be executed alongside them in both Fast and Deep modes.

(Previously: Only 11 built-in diagnostic checks existed. Plugin checks let third parties add custom validation.)

#### Scenario: Plugin Health Check Runs in Deep Mode
- GIVEN a plugin registers a `HealthCheck` validating a custom toolchain installation
- WHEN `DiagnosticEngine::run(DiagnosticMode::Deep)` is called
- THEN the engine MUST execute the plugin health check alongside built-in checks and include its findings in the results

#### Scenario: Plugin Health Check Skipped Mode
- GIVEN a plugin registers a `HealthCheck` that requires network access (Deep-only)
- WHEN `DiagnosticEngine::run(DiagnosticMode::Fast)` is called
- THEN the engine MUST skip the plugin check (by the same Fast/Deep mode rules as built-in checks)
