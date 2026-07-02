# Proposal: Configuration & Secrets Platform

## Intent

Unified, secure, and declarative environment resolution framework. Solves fragmented config management and plaintext secrets storage by implementing a 5-level configuration hierarchy (forge.toml, forge.lock, forge.env, forge.secrets, forge.local.toml) with profile overlays, strict precedence, and secure OS keyring integration or Argon2id/AES-256-GCM fallback encryption.

## Scope

### In Scope
- Config schema validation in `forge.toml` with profile mapping (development, production, ci).
- Variables interpolation supporting derived keys (e.g., `${runtime.python.path}`, `${workspace.root}`).
- Public Engine API facade endpoints for configuration and secret access.
- Fallback cryptography using Argon2id + AES-256-GCM for `forge.secrets`.
- CLI Commands: `forge secret <set|get|list|remove|export|import|doctor>` and `forge env <list|get|set|unset|resolve>`.
- Extensible `SecretProvider` and `ConfigurationProvider` traits.

### Out of Scope
- External cloud provider integrations (AWS Secrets Manager, 1Password).
- External binary helpers for `PluginSecretProvider` (traits defined only).

## Capabilities

### New Capabilities
- `config-engine`: Multi-layered configuration manager and interpolator.
- `secrets-engine`: OS Keyring manager and client-side encryption.
- `config-validation`: Schema enforcement and type checks.
- `config-cli-commands`: Commands env/secret.

### Modified Capabilities
- `runtime-engine-environment`: Materialize process environments using the new resolver.

## Approach

Rust implementation using standard cryptography (`argon2`, `aes-gcm`) and `keyring` crates. Integrate into the CLI parser and core engine environment module.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core` | Modified | Core config resolving logic, traits, crypto engine, and engine facade. |
| `crates/forge-cli` | Modified | Addition of `env` and `secret` CLI subcommands. |
| `openspec/specs` | New | Creation of new capability specs. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| OS Keyring unavailable in headless/CI environments | High | Fall back to file-based Argon2id + AES-256-GCM client-side encryption. |
| Configuration file merge conflicts | Low | Enforce strict 5-level precedence hierarchy and validation schemas. |

## Rollback Plan

Revert git commits back to the previous stable release tag. The schema of `forge.toml` remains backwards-compatible, and any generated `forge.secrets` files can be decrypted via the backup export command or deleted.

## Dependencies

- Rust crates: `argon2`, `aes-gcm`, `keyring-rs`, `serde`.

## Success Criteria

- [ ] Successful parsing of the 5-layered configuration stack with correct precedence.
- [ ] Variables correctly interpolated from environment/workspace paths.
- [ ] Secret CLI set/get commands work seamlessly via OS keyring or fallback crypto.
- [ ] 100% test coverage for the fallback cryptography module.
