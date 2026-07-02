# Pin-by-Hash Specification

## Purpose

Forge.toml runtime entries MAY include an optional `sha256` field to pin artifact downloads by content hash. When set, forge MUST verify the downloaded artifact's SHA-256 hash against the pin before extraction, providing manifest-level supply chain protection alongside the existing lockfile hash.

## Requirements

### Requirement: Pin-by-Hash Verification

When a runtime entry in `forge.toml` specifies a `sha256` value, forge MUST verify the downloaded artifact's SHA-256 hash against the pin BEFORE extraction.

- GIVEN a `forge.toml` runtime entry with `sha256 = "abc123def..."` and the downloaded artifact's actual SHA-256 matches `abc123def...`
- WHEN forge downloads and verifies the artifact
- THEN forge SHALL proceed with normal installation (extract, cache, etc.)

- GIVEN a `forge.toml` runtime entry with `sha256 = "abc123def..."` and the downloaded artifact's actual SHA-256 is `xyz789...`
- WHEN forge verifies the hash
- THEN forge MUST NOT extract the downloaded artifact
- AND MUST emit an error showing both expected (`abc123def...`) and actual (`xyz789...`) hashes
- AND MUST log the mismatch

### Requirement: Manifest Syntax

Forge.toml MUST accept `sha256` in both object syntax (`{ version = "...", sha256 = "..." }`) and string syntax (`"20.11.0"` with no hash — current behavior preserved).

- GIVEN a `forge.toml` runtime entry specified as a string (`node = "20.11.0"`)
- WHEN forge parses the manifest
- THEN forge SHALL use the current behavior (no manifest-level pin, lockfile hash verification applies)
- AND SHALL NOT require a `sha256` field

- GIVEN a `forge.toml` runtime entry with `sha256` set to an invalid hex string
- WHEN forge parses the manifest
- THEN forge SHALL emit a validation error
- AND SHALL reject the configuration

### Requirement: Non-Destructive on Mismatch

On hash mismatch, forge MUST NOT delete, overwrite, or modify the downloaded file on disk beyond leaving it unextracted.

- GIVEN a hash mismatch during verification
- WHEN forge handles the error
- THEN the downloaded artifact file SHALL remain in the download cache (if already stored)
- AND SHALL NOT be extracted or installed

### Requirement: Compatibility with Existing Behavior

Omitting `sha256` MUST produce identical behavior to current forge versions — lockfile-based hash verification during extraction still applies.

- GIVEN a `forge.toml` without any `sha256` fields
- WHEN forge installs runtimes
- THEN forge SHALL behave identically to the version before this feature
- AND lockfile hash verification SHALL still occur during extraction
