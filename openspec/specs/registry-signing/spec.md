# Registry Signing Specification

## Purpose

FRRS metadata MUST be verifiable via detached GPG signatures before forge trusts remote registry data. This prevents man-in-the-middle attacks and tampered metadata from compromising resolved toolchains.

## Requirements

### Requirement: GPG Signature Verification

Every FRRS `metadata.toml` MUST have a companion `metadata.toml.asc` detached GPG signature. The RemoteRegistry MUST verify the signature against a trusted keyring before using any data from the fetch.

- GIVEN a remote registry fetch returns `metadata.toml` and `metadata.toml.asc`
- WHEN forge verifies the GPG signature against a trusted key
- AND the signature is valid
- THEN forge SHALL accept and cache the metadata

- GIVEN a remote registry fetch returns `metadata.toml` and `metadata.toml.asc`
- WHEN forge verifies the GPG signature against all trusted keys
- AND the signature is invalid or the signer is untrusted
- THEN forge SHALL discard the fetched data
- AND SHALL fall back to cached metadata (if available)
- AND SHALL log a warning with the verification failure reason

- GIVEN a remote registry fetch where `metadata.toml.asc` is missing
- WHEN forge is operating in strict mode
- THEN forge SHALL reject the metadata
- AND SHALL fall back to cached metadata (if available)
- AND SHALL log an error

- GIVEN a remote registry fetch where `metadata.toml.asc` is missing
- WHEN forge is operating in lenient mode
- THEN forge MAY accept the metadata
- AND SHALL log a warning about the missing signature

### Requirement: Trusted Keyring

Forge MUST embed a hardcoded public key for `registry.forge.sh` in the binary. Additional trusted keys MUST be accepted via the `FORGE_TRUSTED_KEYS` environment variable (armored GPG keys, newline or semicolon separated).

- GIVEN forge is configured with `FORGE_TRUSTED_KEYS` containing an additional armored public key
- WHEN forge verifies metadata signed with that additional key
- THEN forge SHALL accept the signature as valid

- GIVEN forge is configured with `FORGE_TRUSTED_KEYS` containing an invalid or malformed key
- WHEN forge initializes the trusted keyring
- THEN forge SHALL log a warning
- AND SHALL skip the malformed entry

### Requirement: No Downloaded Code Execution

Forge MUST NOT execute downloaded files during metadata verification or any registry operation.

- GIVEN forge downloads FRRS metadata and signature files
- WHEN processing the downloaded files
- THEN forge MUST NOT execute any downloaded content

### Requirement: Verification Failure Logging

Forge SHOULD log all verification failures with sufficient detail for debugging (key ID, fingerprint, reason, registry URL).

- GIVEN a signature verification fails due to an unknown key
- THEN forge SHALL log the key ID, registry URL, and failure reason
