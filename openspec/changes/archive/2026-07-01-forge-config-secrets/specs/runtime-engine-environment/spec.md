# Runtime Engine Environment Delta Specification

## Change: forge-config-secrets

## Purpose

Modify environment activation to route env materialization through the new 5-level configuration resolver.

## Modified Requirements

### Requirement: Environment Materialization (Modifies REQ-ENV-001, REQ-ENV-002)

The runtime engine MUST materialize process environments by executing the 5-layered configuration resolver, combining manifest configurations, environment files, secrets, local overrides, and CLI overrides instead of parsing `forge.env` in isolation.

#### Scenario: Environment Materialization routing
- GIVEN a runtime environment request
- WHEN the process starts
- THEN the system MUST resolve environment variables from the 5-layer precedence stack, applying validation and variable interpolation before injection
