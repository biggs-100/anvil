# Configuration CLI Commands Specification

## Purpose

Define the user interface CLI subcommands for managing configurations and secrets in Forge.

## Requirements

### Requirement: Secrets CLI Commands

The system MUST provide a CLI subcommand structure for secrets management:
- `forge secret set <key> <value>`: Store a secret using the configured provider.
- `forge secret get <key>`: Retrieve and decrypt a secret value.
- `forge secret list`: List secret keys (values MUST be masked).
- `forge secret remove <key>`: Delete a secret key from the provider.
- `forge secret export`: Export decrypted secrets to backup JSON/TOML (requires authentication/confirmation).
- `forge secret import <file>`: Import secrets into the provider.
- `forge secret doctor`: Verify secrets integrity/provider health.

#### Scenario: Set and Get a Secret
- GIVEN the CLI environment is initialized
- WHEN the user executes `forge secret set API_KEY "secret_val"` and then `forge secret get API_KEY`
- THEN the command MUST complete successfully and output `secret_val` to stdout

#### Scenario: List Secrets Masks Value
- GIVEN a secret `STRIPE_KEY` is set to `sk_test_5123`
- WHEN the user executes `forge secret list`
- THEN the system MUST print `STRIPE_KEY` with the value omitted or masked (e.g. `[REDACTED]`)

---

### Requirement: Env CLI Commands

The system MUST provide a CLI subcommand structure for environment management:
- `forge env list`: List all active/materialized environment variables.
- `forge env get <key>`: Print the materialized value of a specific key.
- `forge env set <key> <value>`: Set a local environment override in `forge.local.toml`.
- `forge env unset <key>`: Remove a local environment override from `forge.local.toml`.
- `forge env resolve`: Pre-compute and print the entire 5-layered environment block.

#### Scenario: Set Local Env Override
- GIVEN `forge.local.toml` does not contain `LOG_LEVEL`
- WHEN the user executes `forge env set LOG_LEVEL debug`
- THEN the system MUST write `LOG_LEVEL = "debug"` to `forge.local.toml`
