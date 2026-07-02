# Design: Configuration & Secrets Platform

This document describes the technical implementation design for the Forge unified 5-layer environment configuration and secrets platform.

## Technical Approach

Forge's environment resolution is refactored from a simple `forge.env` parser into a 7-layered resolver supporting profile overlays, declarative schema validation, and variable interpolation. Secrets are integrated natively using the OS keyring with a secure, workspace-bound AES-256-GCM fallback encryptor.

## Architecture Decisions

| Decision Area | Option | Tradeoff | Decision |
| :--- | :--- | :--- | :--- |
| **Workspace Paths** | Direct query to `Engine` vs `RuntimeContextProvider` trait | Circular dependencies between core modules; trait decouples resolver. | Use `RuntimeContextProvider` trait. |
| **Fallback Crypto** | Keyring fallback to plaintext vs AES-256-GCM | Plaintext lacks security; AES-256-GCM with Argon2id provides CI and headless safety. | Encrypted fallback using Argon2id + AES-256-GCM. |
| **AAD Binding** | Key only vs Workspace ID binding | Keys could be moved between projects; Workspace ID AAD prevents cross-project file decryption. | Bind authenticated encryption (AAD) to workspace ID. |

## Data Flow

```
   CLI Flags / System Env / Local TOML / forge.secrets / forge.env / Profiles / Manifest
                                      │
                                      ▼
                        [7-Layer Precedence Resolver]
                                      │
                                      ▼
                   [Interpolator (${workspace.root}, etc.)]
                                      │
                                      ▼
                       [Declarative Schema Validation]
                                      │
                                      ▼
                          [Materialized Environment]
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/secrets/mod.rs` | Create | Traits `SecretProvider`, `ConfigurationProvider`, keyring and fallback encryptor. |
| `crates/forge-core/src/environment.rs` | Modify | Define `RuntimeContextProvider` and route materialization to the resolver. |
| `crates/forge-core/src/resolver.rs` | Modify | Implement the 7 precedence levels resolver and interpolator. |
| `crates/forge-cli/src/main.rs` | Modify | Add `env` and `secret` subcommands, map validation errors in `doctor`. |

## Interfaces / Contracts

```rust
pub trait SecretProvider: Send + Sync {
    fn name(&self) -> &str;
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
    fn list(&self) -> Result<Vec<String>, String>;
}

pub trait ConfigurationProvider: Send + Sync {
    fn name(&self) -> &str;
    fn load(&self, ctx: &dyn RuntimeContextProvider) -> Result<HashMap<String, String>, String>;
}

pub trait RuntimeContextProvider: Send + Sync {
    fn workspace_root(&self) -> &Path;
    fn runtime_path(&self, name: &str) -> Option<PathBuf>;
}

pub enum ValueSource {
    CliOverride,
    SystemEnv,
    LocalOverride,
    SecretProvider(String),
    EnvFile,
    ProfileOverlay(String),
    DefaultManifest,
}

pub struct VarMetadata {
    pub key: String,
    pub source: ValueSource,
}

pub struct ResolvedEnvironment {
    pub vars: HashMap<String, String>,
    pub metadata: HashMap<String, VarMetadata>,
}
```

## Resolving Pipeline and Interpolation

Precedence is evaluated from Level 1 (highest) to Level 7 (lowest):
1. **Level 1 (CLI Override):** Values passed explicitly via `--env KEY=VAL`.
2. **Level 2 (System Env):** Environment variables matching prefix `FORGE_VAR_<KEY>`.
3. **Level 3 (Local Overrides):** Defined in `forge.local.toml`.
4. **Level 4 (Secrets Providers):** Mapped secrets from `forge.secrets` via OS Keyring/Fallback.
5. **Level 5 (Env File):** Key-value pairs defined in `forge.env`.
6. **Level 6 (Profile Overlays):** Block `[profile.<active_profile>.env]` from `forge.toml`.
7. **Level 7 (Defaults):** Default value declarations in `[config.definitions]` schema inside `forge.toml`.

### Interpolation Logic
Variable interpolation scans resolved strings for `${pattern}`:
- `${workspace.root}`: Replaced by workspace root path via `RuntimeContextProvider`.
- `${runtime.<name>.path}`: Replaced by the tool's installation folder.
- `${env.KEY}`: Replaced by system env `KEY`.

## Cryptography and Keyring Platform

- **OS Keyring:** Primary provider using `keyring` crate (targets Service: `forge-secrets`, Account: `workspace_id::key`).
- **Fallback Encryptor:** Used if OS Keyring returns error or CI is headless.
  - **Key Derivation (KDF):** Argon2id, 64MB memory, 3 iterations, 16-byte random salt.
  - **Encryption:** AES-256-GCM. Workspace ID from `forge.toml` is passed as AAD.
  - **CI Bypass:** If `FORGE_MASTER_KEY` environment variable is set, it is used directly as the passphrase, skipping the password prompt.

## CLI Commands and Validation

### CLI Commands (main.rs)
- `forge env <list | get <key> | set <key> <value> | unset <key> | resolve>`
- `forge secret <set <key> <value> | get <key> | list | remove <key> | export | import <file> | doctor>`

### Validation Mapping
Schemas from `forge.toml` define `type` (`string`/`integer`/`boolean`), `required`, and `pattern` (regex).
Validation is run:
1. During environment materialization. If invalid, the process rejects loading.
2. In `forge doctor` (both human and AI subcommands) to populate `DoctorIssue` structures:
```rust
struct DoctorIssue {
    id: String,
    severity: String, // "critical" | "warning"
    tool: String,     // "config"
    message: String,  // e.g. "DATABASE_URL does not match regex pattern"
    remediation: String,
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Fallback Crypto | Assert AES-GCM encryption works, verify key derivation from `FORGE_MASTER_KEY`, ensure decryption fails on wrong AAD workspace ID. |
| Unit | Resolving Pipeline | Setup mock providers for Levels 1–7; assert precedence correctness and interpolation resolution. |
| Mock | OS Keyring | Stub `SecretProvider` interface to verify seamless fallback transitions when keyring is locked/unavailable. |
| Integration | Schema Validation | Materialize configurations with missing required keys, invalid types, and pattern mismatches, asserting `DoctorIssue` formatting. |

## Rollback Plan

- **Code:** Revert to previous stable tag.
- **Secrets:** Keep previous secrets. Any generated `forge.secrets` files remain compatible or can be decrypted using the exported JSON backup.
