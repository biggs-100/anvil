# Design: Real Runtime Downloads & Offline-First Registry

## Technical Approach

Decouple toolchain providers and archive extraction mechanisms through structured traits. An offline-first local hybrid registry metadata database (`.anvil/metadata_cache.toml`) will handle offline semver matching. Windows ARM64 missing architectures fall back to emulated x86_64 versions, logging details to `anvil.lock`. A Tokio `JoinSet` handles asynchronous concurrent download, verification, and extraction pipelines.

## Architecture Decisions

| Decision / Option | Tradeoffs | Selected Choice & Rationale |
| :--- | :--- | :--- |
| **Offline Version Resolution**: Static registry vs. Hybrid Registry | Static has zero runtime overhead but requires CLI updates. Hybrid is complex but matches offline using cached entries. | **Hybrid Registry**: Uses local cache `.anvil/metadata_cache.toml` with remote fallback to allow updates. |
| **Extraction Execution**: Fork system tools vs. Trait-based native decoders | Forking system tools reduces binary size but has runtime dependencies. Trait decoders are self-contained. | **Trait-based `Extractor`**: Encapsulates `zip`, `flate2`, and `xz2` natively to guarantee cross-platform predictability. |
| **Windows ARM64 Fallback**: Hard fail vs. Emulated execution | Failing blocks developers on ARM64. Emulation via x86_64 keeps them operational with transparent audit logs. | **Emulated x86_64 Fallback**: Fallback to x86_64 binaries and log emulation details to `anvil.lock`. |

## Data Flow

```text
[anvil.toml] ──> [Runtime Manager] ──> [Provider] ──> [Hybrid Registry] (local/remote)
                       │
                       ├──(Parallel Spawn via Tokio JoinSet)
                       │        │
                       │        ├──> [Downloader] ──> [SHA-256 Verify] 
                       │        │
                       │        └──> [Extractor] (Zip/TarGz/TarXz) ──> [Local Cache]
                       ▼
                 [anvil.lock] (Updated with Runtime Info & Emulation Logs)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/Cargo.toml` | Modify | Add `xz2` dependency for TarXz extraction. |
| `crates/anvil-core/src/lib.rs` | Modify | Add `Extractor` and `Provider` traits, implementation structs, and `HybridRegistry`. |
| `crates/anvil-core/src/lock.rs` | Create | Refactor `Lockfile` and serialization logic, adding `EmulationLog`. |
| `crates/forge-runtime/src/manager.rs` | Modify | Update `runtime-manager` orchestration for concurrency using `JoinSet`. |

## Interfaces / Contracts

```rust
pub trait Extractor: Send + Sync {
    fn extract(&self, archive: &Path, dest: &Path) -> Result<(), String>;
}

pub struct ZipExtractor;
pub struct TarGzExtractor;
pub struct TarXzExtractor; // links to xz2

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String>;
}

pub struct NodeProvider;
pub struct PythonProvider;
pub struct BunProvider;
pub struct GoProvider;
pub struct RustProvider;

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct HybridRegistry {
    pub runtimes: Vec<RegistryEntry>,
}
```

### Emulation Log Structure in `anvil.lock`
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmulationLog {
    pub requested: String,
    pub installed: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeLock {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emulation: Option<EmulationLog>,
}
```

## Offline Version Matching

To match a loose version range (e.g. `^20`) offline:
1. Parse the range string into a `semver::VersionReq`.
2. Load entries from `.anvil/metadata_cache.toml`.
3. Filter entries matching target `name`, `platform`, and `arch`.
4. Filter by evaluating each candidate version using `VersionReq::matches`.
5. Select and return the highest matching candidate. If none exist, return an error.

## Parallel Orchestration Logic

The `runtime-manager` implements concurrent operations:
1. Pre-resolves all requested runtimes through their respective `Provider`.
2. Scans for required fallback emulation for Windows ARM64.
3. Spawns tasks via `tokio::task::JoinSet`:
   - Each task downloads the toolchain, verifies the SHA-256 checksum, and triggers the mapped `Extractor`.
4. If any task returns an `Err`, invoke `JoinSet::abort_all()` to cancel other tasks, clean up partial files, and propagate the error.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Extractor Trait Path Traversal | Test extraction of archives containing files with `../` path components to ensure rejection. |
| Unit | Offline Matching | Test range matching (`^20`, `~1.8`) against mock registry structures. |
| Integration | Concurrency & Failures | Setup mock download server. Spawn concurrent requests, trigger failure on one, verify total cancelation. |
| Integration | Lockfile Serialization | Verify Windows ARM64 fallback triggers correct `emulation` fields in `anvil.lock`. |

## Migration / Rollout

No migration required. If the local metadata cache is corrupted, delete `.anvil/metadata_cache.toml` to trigger rebuild.
