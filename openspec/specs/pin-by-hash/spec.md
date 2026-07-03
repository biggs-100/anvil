# Pin-by-Hash Specification

## Purpose

anvil.toml runtime entries MAY include an optional `sha256` field to pin artifact downloads by content hash. When set, anvil MUST verify the downloaded artifact's SHA-256 hash against the pin before extraction, providing manifest-level supply chain protection alongside the existing lockfile hash.

## Requirements

### Requirement: Pin-by-Hash Verification

When a runtime entry in `anvil.toml` specifies a `sha256` value, anvil MUST verify the downloaded artifact's SHA-256 hash against the pin BEFORE extraction.

- GIVEN a `anvil.toml` runtime entry with `sha256 = "abc123def..."` and the downloaded artifact's actual SHA-256 matches `abc123def...`
- WHEN anvil downloads and verifies the artifact
- THEN anvil SHALL proceed with normal installation (extract, cache, etc.)

- GIVEN a `anvil.toml` runtime entry with `sha256 = "abc123def..."` and the downloaded artifact's actual SHA-256 is `xyz789...`
- WHEN anvil verifies the hash
- THEN anvil MUST NOT extract the downloaded artifact
- AND MUST emit an error showing both expected (`abc123def...`) and actual (`xyz789...`) hashes
- AND MUST log the mismatch

### Requirement: Manifest Syntax

anvil.toml MUST accept `sha256` in both object syntax (`{ version = "...", sha256 = "..." }`) and string syntax (`"20.11.0"` with no hash — current behavior preserved).

- GIVEN a `anvil.toml` runtime entry specified as a string (`node = "20.11.0"`)
- WHEN anvil parses the manifest
- THEN anvil SHALL use the current behavior (no manifest-level pin, lockfile hash verification applies)
- AND SHALL NOT require a `sha256` field

- GIVEN a `anvil.toml` runtime entry with `sha256` set to an invalid hex string
- WHEN anvil parses the manifest
- THEN anvil SHALL emit a validation error
- AND SHALL reject the configuration

### Requirement: Non-Destructive on Mismatch

On hash mismatch, anvil MUST NOT delete, overwrite, or modify the downloaded file on disk beyond leaving it unextracted.

- GIVEN a hash mismatch during verification
- WHEN anvil handles the error
- THEN the downloaded artifact file SHALL remain in the download cache (if already stored)
- AND SHALL NOT be extracted or installed

### Requirement: Compatibility with Existing Behavior

Omitting `sha256` MUST produce identical behavior to current anvil versions — lockfile-based hash verification during extraction still applies.

- GIVEN a `anvil.toml` without any `sha256` fields
- WHEN anvil installs runtimes
- THEN anvil SHALL behave identically to the version before this feature
- AND lockfile hash verification SHALL still occur during extraction
