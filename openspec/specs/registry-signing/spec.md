# Registry Signing Specification

## Purpose

ARRS metadata MUST be verifiable via detached GPG signatures before anvil trusts remote registry data. This prevents man-in-the-middle attacks and tampered metadata from compromising resolved toolchains.

## Requirements

### Requirement: GPG Signature Verification

Every ARRS `metadata.toml` MUST have a companion `metadata.toml.asc` detached GPG signature. The RemoteRegistry MUST verify the signature against a trusted keyring before using any data from the fetch.

- GIVEN a remote registry fetch returns `metadata.toml` and `metadata.toml.asc`
- WHEN anvil verifies the GPG signature against a trusted key
- AND the signature is valid
- THEN anvil SHALL accept and cache the metadata

- GIVEN a remote registry fetch returns `metadata.toml` and `metadata.toml.asc`
- WHEN anvil verifies the GPG signature against all trusted keys
- AND the signature is invalid or the signer is untrusted
- THEN anvil SHALL discard the fetched data
- AND SHALL fall back to cached metadata (if available)
- AND SHALL log a warning with the verification failure reason

- GIVEN a remote registry fetch where `metadata.toml.asc` is missing
- WHEN anvil is operating in strict mode
- THEN anvil SHALL reject the metadata
- AND SHALL fall back to cached metadata (if available)
- AND SHALL log an error

- GIVEN a remote registry fetch where `metadata.toml.asc` is missing
- WHEN anvil is operating in lenient mode
- THEN anvil MAY accept the metadata
- AND SHALL log a warning about the missing signature

### Requirement: Trusted Keyring

Anvil MUST embed a hardcoded public key for `registry.anvil.dev` in the binary. Additional trusted keys MUST be accepted via the `ANVIL_TRUSTED_KEYS` environment variable (armored GPG keys, newline or semicolon separated).

- GIVEN anvil is configured with `ANVIL_TRUSTED_KEYS` containing an additional armored public key
- WHEN anvil verifies metadata signed with that additional key
- THEN anvil SHALL accept the signature as valid

- GIVEN anvil is configured with `ANVIL_TRUSTED_KEYS` containing an invalid or malformed key
- WHEN anvil initializes the trusted keyring
- THEN anvil SHALL log a warning
- AND SHALL skip the malformed entry

### Requirement: No Downloaded Code Execution

Anvil MUST NOT execute downloaded files during metadata verification or any registry operation.

- GIVEN anvil downloads ARRS metadata and signature files
- WHEN processing the downloaded files
- THEN anvil MUST NOT execute any downloaded content

### Requirement: Verification Failure Logging

Anvil SHOULD log all verification failures with sufficient detail for debugging (key ID, fingerprint, reason, registry URL).

- GIVEN a signature verification fails due to an unknown key
- THEN anvil SHALL log the key ID, registry URL, and failure reason
