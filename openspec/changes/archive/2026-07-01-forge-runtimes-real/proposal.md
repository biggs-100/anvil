# Proposal: Real Runtime Downloads & Offline-First Registry

## Intent
Implement real runtime downloads, resolving versions dynamically through an updateable offline-first registry, concurrent toolchain downloading, and a modular provider/archive extraction architecture.

## Scope

### In Scope
- **Runtime Providers**: Decouple languages (Python, Node, Bun, Go, Rust) from the core engine.
- **Hybrid Registry**: Use normalized internal metadata catalogs (no direct GitHub scraping).
- **Archive Extractors**: Structured traits supporting ZIP, TarGz, TarXz formats (via `xz2` crate).
- **Dynamic Resolving**: Exact offline version request fails if uncached; loose ranges (e.g. `^20`) resolve to compatible cached assets.
- **Windows ARM64**: Fallback to x86_64 emulation when native ARM64 is missing, logged in `forge.lock` (requested, installed, reason fields).
- **Parallelism**: Asynchronous parallel download, verification, and extraction using `tokio`.

### Out of Scope
- Remote updates for system packages (strictly local).
- Secure checksum signature validation keys (deferred).

## Capabilities

### New Capabilities
- `runtime-providers`: Abstracts language-specific resolution, download asset mapping, and installation checks.
- `archive-extractors`: Trait-based extraction interface mapping Zip, TarGz, and TarXz (using xz2) file types.
- `hybrid-registry`: Internal registry metadata coordinator and local `.forge/metadata_cache.toml` cache manager.

### Modified Capabilities
- `runtime-manager`: Integrate with providers and extractors to orchestrate parallel downloads.
- `lockfile-generator`: Support recording platform emulation details.

## Approach
- Integrate `xz2` crate for TarXz extraction.
- Define `Extractor` and `Provider` traits in `crates/forge-core`.
- Implement asynchronous parallel downloads using `tokio`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core` | Modified | Add `Extractor` and `Provider` traits, integrate `xz2` |
| `crates/forge-runtime` | Modified | Update `runtime-manager` orchestration logic |
| `crates/forge-lock` | Modified | Update lockfile to support emulation recording |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| TarXz compilation failures on target | Low | Ensure standard build environment or pre-built C library options for `lzma-sys` |
| Concurrency overhead/deadlocks | Low | Use standard `tokio` channels and join handles with proper timeouts |

## Rollback Plan
Revert code changes in `crates/` to previous commit. Remove caching in `.forge/metadata_cache.toml` if it gets corrupted.

## Dependencies
- `xz2` crate for LZMA/XZ support.
- `tokio` (existing for async handling).

## Success Criteria
- [ ] Concurrent downloads execute without blocking the main event loop.
- [ ] System resolves and extracts ZIP, TarGz, and TarXz (with xz2) successfully.
- [ ] Emulated Windows ARM64 fallbacks correctly logged to `forge.lock`.
- [ ] Offline loose version resolution succeeds if matches exist in `.forge/metadata_cache.toml`.
