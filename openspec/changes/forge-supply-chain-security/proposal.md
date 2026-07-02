# Proposal: Forge Supply Chain Security

## Intent

FRRS metadata is served unsigned, forge.toml cannot pin by hash, and there's no audit trail of what forge downloads. Three security gaps that undermine trust in resolved toolchains.

## Scope

### In Scope
- GPG signature verification for FRRS `metadata.toml` (detached `.asc` files, trusted keyring)
- Optional pin-by-hash in `forge.toml` manifest per runtime entry
- `forge audit` CLI command — download/operation history in table + JSON format

### Out of Scope
- Key rotation/expiry management — first version uses static keys
- Automatic GPG key fetching from keyservers
- Tamper-proof journal (append-only is enough for v1)
- Signing forge.lock or other artifacts

## Capabilities

### New Capabilities
- `supply-chain-security`: GPG verification of FRRS metadata, pin-by-hash in manifests, `forge audit` CLI

### Modified Capabilities
- `frrs-spec`: FRRS metadata SHALL include a `.asc` companion, verification SHALL occur before trust
- `hybrid-registry`: RemoteRegistry SHALL verify GPG signature before caching metadata; fallback to cached on failure
- `cli-commands-lifecycle`: SHALL add `audit` command — reads journal for download operations
- `observability-api-v1`: Engine facade SHALL expose `audit()` method
- `config-engine` / `config-validation`: forge.toml runtime entries SHOULD accept optional `sha256` field
- `runtime-engine-installer`: SHALL verify artifact hash against manifest pin before extraction
- `observability-introspection` / `observability-journal`: journal schema SHALL include download URLs, sizes, verification status

## Approach

**1. GPG Signing**: Add `pgp` crate dependency. On remote metadata fetch, request `metadata.toml.asc`, verify against hardcoded `registry.forge.sh` key + `FORGE_TRUSTED_KEYS` env var. Invalid/missing sig → log warning, serve cached data.

**2. Pin by Hash**: Extend forge.toml `[runtimes]` entry parsing to accept inline `sha256`. Installer verifies hash after download, before extraction. Error on mismatch.

**3. Audit Log**: New `forge audit` subcommand. Reads `.forge/journal.jsonl`, filters for download/install operations, formats as table (default) or JSON (`--json`). Columns: timestamp, runtime, version, URL, size, SHA-256, verified status.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/registry/remote.rs` | Modified | GPG verification before cache |
| `crates/forge-core/src/config/manifest.rs` | Modified | Pin-by-hash parsing |
| `crates/forge-core/src/installer.rs` | Modified | Hash verification step |
| `crates/forge-cli/src/commands/` | New | `audit` command module |
| `crates/forge-core/src/api/v1.rs` | Modified | `audit()` method |
| `openspec/specs/` | Multiple | Delta specs per modified capability |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `pgp` crate maturity / cross-compile issues | Low | Evaluate alternatives (`sequoia-pgp`, `gpgme`); fallback to `gpg` CLI |
| GPG key distribution — user confusion | Med | Clear error messages, documented env var (`FORGE_TRUSTED_KEYS`) |
| Hash mismatch blocks all usage | Low | Pin-by-hash is opt-in; existing behavior unchanged when omitted |

## Rollback Plan

- Revert `registry/remote.rs` — metadata fetched without GPG check
- Remove sha256 field from manifest parsing — pin-by-hash only in lockfile
- Remove `audit` CLI command — revert `v1.rs` facade

## Dependencies

- `pgp` crate (or `sequoia-openpgp` / `gpgme` — evaluate in design)
- No external services — keyring embedded in binary or via env var

## Success Criteria

- [ ] Remote metadata fetch fails closed on invalid GPG signature (falls to cache)
- [ ] `forge.toml` with `sha256` causes install to fail on hash mismatch
- [ ] `forge.toml` without `sha256` works identically to today
- [ ] `forge audit` prints download history with timestamps, URLs, sizes, hashes
- [ ] `forge audit --json` outputs valid JSON
