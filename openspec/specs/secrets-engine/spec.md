# Secrets Engine Specification

## Purpose

Define secure secrets storage and retrieval mechanisms, supporting OS keyring integration, extensible providers, and encrypted local fallback storage.

## Requirements

### Requirement: Secret Resolution and Provider Trait

The secrets engine MUST define a `SecretProvider` trait to perform retrieval, storage, and deletion of secret keys. The core system MUST dynamically query registered providers based on the `anvil.secrets` mapping.

#### Scenario: Resolve Secret via Registered Provider
- GIVEN a mapping of `STRIPE_KEY` pointing to a mock provider with value `sk_test_123`
- WHEN the system requests `STRIPE_KEY`
- THEN the system MUST query the mock provider and return `sk_test_123`

---

### Requirement: OS Keyring Integration

The system MUST support native OS keyring integration (macOS Keychain, Windows Credential Manager, Linux Secret Service via DBus) as the primary provider backend.

#### Scenario: Keyring Retrieve Success
- GIVEN a secret key `DB_PASS` stored in the Windows Credential Manager
- WHEN the keyring provider queries `DB_PASS`
- THEN the system MUST return the decrypted string from Windows Credential Manager

---

### Requirement: Fallback Encryption

When OS Keyrings are unavailable (e.g. in CI or headless environments), the system MUST fallback to file-based client-side encryption.
- **Key Derivation (KDF):** Argon2id with 16-byte random salt, 64MB memory cost, 3 iterations, and 4 threads parallelism.
- **Encryption:** AES-256-GCM.
- **Passphrase bypass:** If `ANVIL_MASTER_KEY` environment variable is set, it MUST be consumed as the passphrase, bypassing interactive prompts.
- **Integrity (AAD):** Additional Authenticated Data MUST bind the encrypted payload to the active workspace ID.

#### Scenario: Encrypted Fallback Using Env Passphrase
- GIVEN a headless environment with `ANVIL_MASTER_KEY` set to `superpass` and an encrypted `anvil.secrets` file
- WHEN a secret is requested
- THEN the system MUST decrypt the secrets file using Argon2id + AES-256-GCM without prompting the user
