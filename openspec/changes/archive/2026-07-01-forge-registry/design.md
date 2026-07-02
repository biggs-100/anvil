# Design: Forge Runtime Registry (FRRS) + Remote Client

## Technical Approach

Extend `HybridRegistry` with a 4-tier resolution chain: (1) loaded flat entries, (2) local FRRS cache directory, (3) remote HTTP fetch, (4) embedded `default_with_internal()` fallback. Add `RemoteRegistry` struct for HTTP + cache. `Resolver` and all `RuntimeProvider` impls remain unchanged — they already delegate to `registry.resolve()`. No new crate deps (reqwest, serde, toml already present).

## Architecture Decisions

### Registry URL Config: forge.toml + env var override
- **Primary**: `[registry] url = "..."` in forge.toml
- **Override**: `FORGE_REGISTRY_URL` env var beats config
- **Default**: `https://registry.forge.sh`
- **Rationale**: Config per project, env for CI, default for zero-config.

### Cache Format: FRRS directory structure
- **Primary cache**: `.forge/metadata_cache/{name}/{version}/metadata.toml`
- **Legacy**: Flat `metadata_cache.toml` kept as first chain link (backward compat)
- **Rationale**: Directory mirrors remote layout; inspectable offline.

### Cache TTL: 24h via file mtime
- Stale entries served when remote unreachable (spec REQ-REG-009)
- Configurable via `[registry] ttl_hours`
- **Rationale**: Zero-overhead expiry, no extra metadata file needed.

### Provider Migration: none needed
- All 5 providers already call `registry.resolve()` — no hardcoded URLs
- Hardcoded URLs live only in `default_with_internal()` (last chain link)

## Data Flow

```
resolve() → self.runtimes (flat file/inline)
          → .forge/metadata_cache/{name}/{version}/metadata.toml
          → RemoteRegistry HTTP GET {base_url}/{name}/{version}/metadata.toml
          → default_with_internal()
          → error

Cache: mtime < TTL → serve cached | mtime stale → refresh, serve stale on fail
Remote: success → save to cache | failure → log, continue chain
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/registry.rs` | Modify | Add `RemoteRegistry`, `FrrsMetadata`, `FrrsIndex` types; extend `HybridRegistry` resolve chain |
| `crates/forge-core/src/resolver.rs` | Modify | Update `resolve_runtime_lock()` to use new registry chain; add `with_remote()` helper |
| `crates/forge-core/Cargo.toml` | Modify | No new deps needed (reqwest, serde, toml already present) |

## Interfaces / Contracts

```rust
// --- FRRS types ---
pub struct FrrsMetadata {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub artifacts: Vec<FrrsArtifact>,
    pub dependencies: Option<Vec<String>>,
}

pub struct FrrsArtifact {
    pub platform: String,
    pub arch: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

// --- RemoteRegistry ---
pub struct RemoteRegistry {
    base_url: String,
    cache_dir: PathBuf,
    client: reqwest::Client,
    ttl: Duration,
}

impl RemoteRegistry {
    pub fn new(base_url: &str, cache_dir: PathBuf) -> Self;
    pub fn with_ttl(self, ttl: Duration) -> Self;
    pub async fn fetch_metadata(&self, name: &str, version: &str)
        -> Result<FrrsMetadata, String>;
    pub fn load_cached(&self, name: &str, version: &str)
        -> Option<FrrsMetadata>;
    pub async fn fetch_index(&self) -> Result<FrrsIndex, String>;
}

// --- Extended HybridRegistry ---
pub struct HybridRegistry {
    pub runtimes: Vec<RegistryEntry>,
    remote: Option<RemoteRegistry>,
    cache_dir: Option<PathBuf>,
}
```

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | FRRS parse + unknown field ignore | Deserialize test TOML strings |
| Unit | Cache TTL logic | Set file mtime, verify stale/fresh |
| Unit | Chain fallback order | Mock each layer, verify priority |
| Integration | Remote HTTP with mock server | `wiremock` local server |
| Integration | End-to-end resolve | Real fs + HTTP mock |

## Migration / Rollout

No data migration needed. Existing `metadata_cache.toml` files continue to work as the first chain link (flat entries). Users who want remote registry get it automatically when they configure `[registry] url` in forge.toml. The `remote-registry` feature flag from the proposal is dropped — reqwest is already a hard dep; the remote call is guarded by whether `RemoteRegistry` is configured (no config = skip remote).

## Open Questions

- [ ] Exact public URL for the default `registry.forge.sh` — who hosts and maintains?
- [ ] Should `forge registry refresh` be a separate CLI operation or part of `forge update`?
