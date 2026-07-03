# Proposal: Anvil Supply Chain Security

## Intent

ARRS metadata is served unsigned, anvil.toml cannot pin by hash, and there's no audit trail of what anvil downloads. Three security gaps that undermine trust in resolved toolchains.

## Scope

### In Scope
- GPG signature verification for ARRS `metadata.toml` (detached `.asc` files, trusted keyring)
- Optional pin-by-hash in `anvil.toml` manifest per runtime entry
- `anvil audit` CLI command — download/operation history in table + JSON format

### Out of Scope
- Key rotation/expiry management — first version uses static keys
- Automatic GPG key fetching from keyservers
- Tamper-proof journal (append-only is enough for v1)
- Signing anvil.lock or other artifacts

## Capabilities

### New Capabilities
- `supply-chain-security`: GPG verification of ARRS metadata, pin-by-hash in manifests, `anvil audit` CLI

### Modified Capabilities
- `arrs-spec`: ARRS metadata SHALL include a `.asc` companion, verification SHALL occur before trust
- `hybrid-registry`: RemoteRegistry SHALL verify GPG signature before caching metadata; fallback to cached on failure
- `cli-commands-lifecycle`: SHALL add `audit` command — reads journal for download operations
- `observability-api-v1`: Engine facade SHALL expose `audit()` method
- `config-engine` / `config-validation`: anvil.toml runtime entries SHOULD accept optional `sha256` field
- `runtime-engine-installer`: SHALL verify artifact hash against manifest pin before extraction
- `observability-introspection` / `observability-journal`: journal schema SHALL include download URLs, sizes, verification status

## Approach

**1. GPG Signing**: Add `pgp` crate dependency. On remote metadata fetch, request `metadata.toml.asc`, verify against hardcoded `registry.anvil.dev` key + `ANVIL_TRUSTED_KEYS` env var. Invalid/missing sig → log warning, serve cached data.

**2. Pin by Hash**: Extend anvil.toml `[runtimes]` entry parsing to accept inline `sha256`. Installer verifies hash after download, before extraction. Error on mismatch.

**3. Audit Log**: New `anvil audit` subcommand. Reads `.anvil/journal.jsonl`, filters for download/install operations, formats as table (default) or JSON (`--json`). Columns: timestamp, runtime, version, URL, size, SHA-256, verified status.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/registry/remote.rs` | Modified | GPG verification before cache |
| `crates/anvil-core/src/config/manifest.rs` | Modified | Pin-by-hash parsing |
| `crates/anvil-core/src/installer.rs` | Modified | Hash verification step |
| `crates/anvil-cli/src/commands/` | New | `audit` command module |
| `crates/anvil-core/src/api/v1.rs` | Modified | `audit()` method |
| `openspec/specs/` | Multiple | Delta specs per modified capability |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `pgp` crate maturity / cross-compile issues | Low | Evaluate alternatives (`sequoia-pgp`, `gpgme`); fallback to `gpg` CLI |
| GPG key distribution — user confusion | Med | Clear error messages, documented env var (`ANVIL_TRUSTED_KEYS`) |
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
- [ ] `anvil.toml` with `sha256` causes install to fail on hash mismatch
- [ ] `anvil.toml` without `sha256` works identically to today
- [ ] `anvil audit` prints download history with timestamps, URLs, sizes, hashes
- [ ] `anvil audit --json` outputs valid JSON
