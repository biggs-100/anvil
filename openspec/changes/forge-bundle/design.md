# Design: forge-bundle

## Technical Approach

Add a `bundle` module to `forge-core` with deterministic tar+gzip archive creation and
verification-based restore. Two new CLI subcommands (`Bundle`, `Restore`) follow the existing
clap pattern. Zero new dependencies — `tar`, `flate2`, `sha2`, `hex` are already in
`forge-core/Cargo.toml`.

## Architecture Decisions

| Option | Considered | Decision | Rationale |
|--------|-----------|----------|-----------|
| CLI shape | `forge bundle create/restore` with sub-subcommand | **Standalone `Bundle` / `Restore`** | Simpler dispatch, matches existing top-level commands (`Lock`, `Sync`, etc.). No sub-group needed for a single verb pair. |
| Checksum location | Separate `.forge.sha256` file | **Inside archive as `bundle.sha256`** | Self-verifying archive — one file, no sidecar to lose. Consumer extracts all entries to a temp dir then verifies before applying. |
| Metadata contents | Full config dump, minimal | **Minimal: forge version, timestamp, workspace_id, runtime count, exclusions list** | Audit trail without duplicating forge.toml content. Workspace_id ties bundle to a project if present. |
| Gzip determinism | Default mtime/file-name | **mtime=0, no filename in gzip header** | `GzEncoder` with `write::GzEncoder::new(writer, Compression::default)` — set mtime to `0` via `Header::default()` (already 0) and construct encoder directly to avoid filename header. |
| Extraction flow | In-place replace, temp dir then move | **Temp dir → verify → atomic rename** | Prevents partial corruption on power loss or interrupted restore. Temp dir in same filesystem for fast rename. |

## Data Flow

```
Bundle flow:

  forge.toml ──→ read_bytes ──→ sha256(toml)
  forge.lock ──→ read_bytes ──→ sha256(lock)
                     │
                     ▼
              build metadata.json ──→ sha256(metadata)
                     │
                     ▼
              build bundle.sha256 (checksums + expected)
                     │
                     ▼
              tar::Builder ──→ entries in deterministic order:
                                1. forge.toml
                                2. forge.lock
                                3. metadata.json
                                4. bundle.sha256
                     │
                     ▼
              GzEncoder (mtime=0) ──→ project.forge

Restore flow:

  project.forge ──→ GzDecoder ──→ tar::Archive ──→ extract to temp dir
                     │
                     ▼
              read bundle.sha256 from temp dir
              verify checksums of all other entries
                     │
              ┌──── match? ────┐
              ▼                ▼
           error("hash     rename forge.toml
           mismatch")     + forge.lock to workspace
                                  │
                                  ▼
                           forge up (delegated)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/bundle.rs` | Create | `create_bundle()`, `restore_bundle()`, `verify_checksums()`, `BundleMetadata`, `BundleChecksums` |
| `crates/forge-core/src/lib.rs` | Modify | Add `pub mod bundle;` and re-export key functions |
| `crates/forge-cli/src/main.rs` | Modify | Add `Bundle` / `Restore` variants to `Commands` enum and match arms |

## Interfaces / Contracts

```rust
// crates/forge-core/src/bundle.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleMetadata {
    pub forge_version: String,
    pub created_at: String,            // ISO 8601
    pub workspace_id: Option<String>,
    pub runtime_count: usize,
    pub excluded_patterns: Vec<String>,
}

/// Per-file checksum entry stored inside bundle.sha256.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChecksumEntry {
    pub path: String,
    pub sha256: String,
}

/// The checksum manifest read from bundle.sha256 inside the archive.
pub type BundleChecksums = Vec<ChecksumEntry>;

/// Create a deterministic archive from the workspace.
///
/// 1. Read forge.toml + forge.lock from `workspace_dir`.
/// 2. Compute SHA-256 of each.
/// 3. Build metadata.json.
/// 4. Build bundle.sha256 (serialized JSON list of ChecksumEntry).
/// 5. Write tar (sorted entries: toml, lock, metadata, checksums) → gzip.
pub fn create_bundle(
    workspace_dir: &Path,
    output_path: &Path,
) -> Result<(), BundleError>

/// Restore a bundle into the workspace.
///
/// 1. Decompress gzip → tar.
/// 2. Extract all entries to a temp directory.
/// 3. Read bundle.sha256, verify every entry.
/// 4. Rename forge.toml and forge.lock into workspace_dir.
/// 5. Clean up temp dir.
pub fn restore_bundle(
    bundle_path: &Path,
    workspace_dir: &Path,
) -> Result<(), BundleError>

/// Verify checksums of extracted entries against bundle.sha256.
/// Returns Ok only if every entry matches.
pub fn verify_checksums(
    extract_dir: &Path,
    checksums: &BundleChecksums,
) -> Result<(), BundleError>

#[derive(Debug)]
pub enum BundleError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    ChecksumMismatch { path: String, expected: String, actual: String },
    MissingEntry(String),
    SecretExcluded(String),
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `create_bundle` with mock forge.toml/lock | Create temp workspace with known files, bundle, assert archive contains 4 entries in order, each entry's checksum is correct. |
| Unit | `verify_checksums` | Corrupted entry → `Err(ChecksumMismatch)`. Missing entry → `Err(MissingEntry)`. All match → `Ok(())`. |
| Unit | Gzip determinism | Same inputs → same bytes (compare byte-by-byte). |
| Integration | `restore_bundle` → `forge up` delegation | Bundle a workspace, restore to a clean dir, verify forge.toml and forge.lock exist with correct content. |
| Security | Secrets exclusion | Create `.forge/` directory and `forge.secrets` file inside workspace, bundle, verify they are NOT in archive entries. |

## Migration / Rollout

No migration required. New feature — no existing data format changes. The `.forge` archive is a new artifact type.

## Open Questions

- [ ] Should `forge bundle` output filename default to `{workspace_dir_name}.forge` or always require `--output`?
