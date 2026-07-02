Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Configuration & Secrets Platform

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 800-1000 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Traits, Keyring, Fallback Crypto | PR 1 | Base, tests included |
| 2 | 7-layered Resolver & Schema Validation | PR 2 | Resolver logic, tests |
| 3 | Introspection CLI & Materialization | PR 3 | CLI integration, tests |

## Phase 1: Traits & Cryptography (PR 1)
- [x] 1.1 Add traits `SecretProvider`, `ConfigurationProvider` in new file `crates/forge-core/src/secrets/mod.rs` and export in `crates/forge-core/src/lib.rs`
- [x] 1.2 Implement mock `SecretProvider` and OS keyring integration using `keyring` crate in `crates/forge-core/src/secrets/mod.rs`
- [x] 1.3 Implement fallback encryption module in `crates/forge-core/src/secrets/mod.rs` using `argon2` and `aes-gcm` bound to workspace ID as AAD. Bypass passphrase prompt using `FORGE_MASTER_KEY` environment variable
- [x] 1.4 Add unit tests verifying AES-256-GCM encryption/decryption, Argon2 KDF, correct AAD validation, and CI bypass in `crates/forge-core/src/secrets/mod.rs`

## Phase 2: 7-Layer Resolver & Schema (PR 2)
- [x] 2.1 Define `RuntimeContextProvider` trait in `crates/forge-core/src/environment.rs` and export in `crates/forge-core/src/lib.rs`
- [x] 2.2 Implement 7-layered precedence resolver in `crates/forge-core/src/resolver.rs` resolving Level 1 down to Level 7
- [x] 2.3 Implement variable interpolation matching `${workspace.root}`, `${runtime.<name>.path}`, and `${env.KEY}` in `crates/forge-core/src/resolver.rs`
- [x] 2.4 Implement schema validation checking types, required, and pattern regex in `crates/forge-core/src/resolver.rs`
- [x] 2.5 Add unit tests for resolving precedence, variable interpolation, and schema validation checks in `crates/forge-core/src/resolver.rs`

## Phase 3: CLI & Materialization (PR 3)
- [x] 3.1 Route environment materialization through new resolver in `crates/forge-core/src/environment.rs`
- [x] 3.2 Add CLI subcommands `env` and `secret` mapping all suboptions in `crates/forge-cli/src/main.rs`
- [x] 3.3 Integrate validation error checks into `forge doctor` `DoctorIssue` reports in `crates/forge-cli/src/main.rs`
- [x] 3.4 Add integration tests verifying end-to-end CLI env/secret behavior in `crates/forge-cli/src/main.rs`

