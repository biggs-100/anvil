# Design: Anvil Supply Chain Security

## Technical Approach

Three independent capabilities layered on existing anvil pipeline points — GPG verification at metadata fetch, pin-by-hash at manifest parse + install, and audit at display. No changes to core resolution/lock/sync orchestration.

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| GPG verifier | `gpg` CLI subprocess | Zero new deps, covers Linux/macOS devs. Rejected `sequoia-openpgp` (large compile), `gpgme` (C lib build dep). Degrades to cache-only when GPG absent |
| Key source | Embedded `const &str` + `ANVIL_TRUSTED_KEYS` env var | Static key avoids runtime fetch; env var allows user override |
| Pin parsing | `#[serde(untagged)]` enum `String | {version,sha256}` | Backward-compat with bare strings, serde-native |
| Pin enforcement | Hash check in `install_runtime_transactional` before extraction | Reuses existing `compute_sha256()`, matches lockfile pattern |
| Audit schema | Extend `Event` with optional fields | Backward-compat JSON deser (`#[serde(default)]`), single journal |
| Audit output | Table (default) + `--json` | Matches `anvil history` pattern |

## Data Flow

### GPG Verification
```
fetch_metadata(name, version)
  ├─ GET {base}/{name}/{ver}/metadata.toml
  ├─ GET {base}/{name}/{ver}/metadata.toml.asc
  ├─ gpg --verify --keyring <temp> sig.tmp data.tmp
  │  ├─ exit 0 → cache & return
  │  ├─ exit non-zero → discard, log warning, fallback to load_cached()
  │  └─ gpg not found → log warning, skip verification, serve
  └─ cache miss on fallback → return error
```

### Pin-by-Hash
```
anvil.toml → Deserialize runtimes
  "node" = "20.11.0"              → RuntimeEntry::Bare, no pin
  "node" = { version, sha256 }   → RuntimeEntry::Pinned, hash stored

install_runtime_transactional()
  download → compute_sha256()
  if manifest_pin matches && computed != pin → abort with mismatch error
  (existing lockfile hash check runs regardless)
  → extract
```

### Audit Command
```
anvil audit
  read .anvil/journal.jsonl
  filter: phase in [Sync, Download, Extract, Commit]
  group by operation_id
  display columns: timestamp, runtime, version, operation, url, size, sha256, verified
  --json → JSON array
  empty/missing journal → "No operations recorded", exit 0
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/src/gpg.rs` | Create | Embedded key constant, `verify_gpg_signature()` fn |
| `crates/anvil-core/src/registry.rs` | Modify | Fetch `.asc`, call verifier in `fetch_metadata()` |
| `crates/anvil-core/src/manifest.rs` | Modify | `RuntimeEntry` enum, `HashMap<String, RuntimeEntry>` |
| `crates/anvil-core/src/installer.rs` | Modify | Manifest-pin hash check before extraction |
| `crates/anvil-core/src/types.rs` | Modify | Add optional audit fields to `Event` |
| `crates/anvil-cli/src/audit.rs` | Create | Audit command handler (read journal, format table/JSON) |
| `crates/anvil-cli/src/main.rs` | Modify | Add `Audit` variant, wire in `run_cli()`, add to `BUILTIN_COMMANDS` |

## Interfaces / Contracts

```rust
// gpg.rs
pub const EMBEDDED_PUBLIC_KEY: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----\n...";

pub fn verify_gpg_signature(
    data: &[u8],
    sig: &[u8],
    additional_keys: &[String],
) -> Result<String, String>;  // Ok(key_id) on valid

// manifest.rs
#[derive(Deserialize)]
#[serde(untagged)]
pub enum RuntimeEntry {
    Bare(String),
    Pinned { version: String, sha256: String },
}

// types.rs — Event additions
pub struct Event {
    // existing fields…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}
```

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | GPG verify valid/invalid sig | Temp GNUPGHOME, known key + unsigned data fixture |
| Unit | `RuntimeEntry` deserialization | `toml::from_str` bare string + object syntax |
| Unit | Installer manifest-pin mismatch | `install_runtime_transactional` with wrong pin, assert error |
| Unit | Event optional field roundtrip | JSON ser/de with and without audit fields |
| Integration | `anvil audit` empty journal | No `.anvil/journal.jsonl`, assert "no operations" |
| Integration | GPG fallback to cache | Mock HTTP returns bad sig, assert cached data served |

## Migration / Rollout

No data migration. All three features are opt-in/additive: GPG signs server-side first (client tolerates missing sig in lenient mode), pin-by-hash is an optional anvil.toml field, audit is a new command reading existing journal. Rollback is revert of the relevant files.

## Open Questions

- [ ] `gpg` CLI path detection — probe once at startup or on each verify call?
- [ ] Strict vs lenient mode — env var `ANVIL_GPG_STRICT=1` or config setting?
- [ ] Audit enrichment — should the installer write Events with audit fields, or should the audit command cross-reference lockfile for sha256/URL?
