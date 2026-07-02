# Proposal: Forge Runtime Registry (FRRS) + Remote Client

## Intent

Eliminate hardcoded download URLs inside forge-core by defining an open standard (FRRS) for runtime toolchain metadata, and adding remote registry fetching to `HybridRegistry`. This makes forge registry-driven, offline-capable, and extensible without code changes.

## Scope

### In Scope
- FRRS format spec (TOML-based: metadata.toml, sha256sums.txt, mirrors.json)
- Remote registry HTTP client in forge-core
- Local cache layer (`.forge/registry/`) with freshness checks
- Migration: `HybridRegistry` chains local cache → remote → embedded fallback
- Plumbing: `default_with_internal()` entries become FRRS-compliant fallback data

### Out of Scope
- Registry server or hosting infrastructure
- User accounts, auth, API keys
- New toolchain providers beyond Python, Node, Bun, Go, Rust
- Version negotiation protocol (satisfied by existing semver resolver)

## Capabilities

### New Capabilities
- `frrs-spec`: Forge Runtime Registry Specification — the formal TOML format describing runtime artifacts (name, version, platform, arch, URL, hash, mirrors, license)

### Modified Capabilities
- `hybrid-registry`: Existing `HybridRegistry` gains remote fetching. Resolution chain: local metadata cache → remote FRRS registry → embedded compiled-in fallback. Cache invalidation via TTL or explicit refresh.

## Approach

1. Define FRRS directory structure and all three file schemas (metadata.toml, sha256sums.txt, mirrors.json) in a spec document.
2. Add `RemoteRegistry` struct in `registry.rs`: HTTP GET to `{base_url}/registry/{runtime}/{version}/metadata.toml`, parse, cache to `.forge/registry/`.
3. Modify `HybridRegistry::resolve()`: check local metadata cache first, then `RemoteRegistry`, then `default_with_internal()`.
4. Add `forge registry refresh` CLI command (optional, in scope for tasks).
5. Add `reqwest` as optional dependency behind a `remote-registry` feature flag.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/registry.rs` | Modified | Add `RemoteRegistry`, extend `HybridRegistry` chain |
| `crates/forge-core/src/resolver.rs` | None | Already uses `HybridRegistry`, continues unchanged |
| `crates/forge-core/Cargo.toml` | Modified | Add `reqwest` behind `remote-registry` feature |
| `openspec/specs/hybrid-registry/spec.md` | Updated | Add remote fetch requirements |
| `openspec/specs/frrs-spec/spec.md` | New | FRRS format spec |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Registry URL unreachable | Medium | Local cache + embedded fallback guarantees offline operation |
| FRRS format changes upstream | Low | Spec is forge-owned open standard; versioned in semver |
| Cache staleness (wrong version resolved) | Low | TTL-based invalidation; `forge registry refresh` forces update |

## Rollback Plan

- Feature-flag `remote-registry` (off by default) → disable restores old behavior
- Remove `RemoteRegistry` struct → `HybridRegistry` falls back to `default_with_internal()` as before
- FRRS spec is documentation-only; no rollback needed

## Dependencies

- `reqwest` crate (HTTP client, optional behind feature flag)
- FRRS static files hosted at a public URL (TBD — separate ops concern)

## Success Criteria

- [ ] `HybridRegistry` resolves a runtime from remote registry and caches it locally
- [ ] Offline with cache → resolves cached entry (no network)
- [ ] Offline without cache → falls back to embedded defaults
- [ ] `default_with_internal()` data is FRRS-compliant
- [ ] All existing resolver tests pass unchanged
