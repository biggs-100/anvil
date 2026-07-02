# Delta for Diagnostic Checks

## ADDED Requirements

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
