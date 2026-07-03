# Runtime Engine Environment Specification

## Purpose

Define environment file resolution, key-value parsing, PATH computation rules, sensitive environment variable masking, and environment materialization through the precedence resolver.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-ENV-001 | The system MUST locate the closest `anvil.env` in parent directories (integrated as one layer of the precedence resolver). | MUST |
| REQ-ENV-002 | The system MUST parse `anvil.env` entries, supporting comments and optional quote stripping. | MUST |
| REQ-ENV-003 | The system MUST construct a PATH environment variable by prefixing runtime binary directories to the current system PATH. | MUST |
| REQ-ENV-004 | The system MUST detect sensitive environment variables using case-insensitive naming rules and redact their values in logs. | MUST |
| REQ-ENV-005 | The runtime engine MUST materialize process environments by executing the 7-layered precedence resolver. | MUST |

### Requirement: Environment Configuration

#### Scenario: Parse Environment File
- GIVEN a `anvil.env` containing `DB_PASS="secure123"` and `# comment`
- WHEN parsing is executed
- THEN the system MUST extract key `DB_PASS` with value `secure123` and ignore the comment line

#### Scenario: Masking Sensitive Key
- GIVEN a parsed map with key `API_KEY` and value `super-secret-token`
- WHEN `mask_env_vars` is executed
- THEN the system MUST replace the value with `[REDACTED]`

### Requirement: Environment Materialization

The runtime engine MUST materialize process environments by executing the 7-layered precedence resolver, combining manifest configurations, environment files, secrets, local overrides, and CLI overrides instead of parsing `anvil.env` in isolation.

#### Scenario: Environment Materialization routing
- GIVEN a runtime environment request
- WHEN the process starts
- THEN the system MUST resolve environment variables from the 7-layer precedence stack, applying validation and variable interpolation before injection
