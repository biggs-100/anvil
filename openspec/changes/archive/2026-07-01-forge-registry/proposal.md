# Proposal: Anvil Runtime Registry (ARRS) + Remote Client

## Intent

Eliminate hardcoded download URLs inside anvil-core by defining an open standard (ARRS) for runtime toolchain metadata, and adding remote registry fetching to `HybridRegistry`. This makes anvil registry-driven, offline-capable, and extensible without code changes.

## Scope

### In Scope
- ARRS format spec (TOML-based: metadata.toml, sha256sums.txt, mirrors.json)
- Remote registry HTTP client in anvil-core
- Local cache layer (`.anvil/registry/`) with freshness checks
- Migration: `HybridRegistry` chains local cache → remote → embedded fallback
- Plumbing: `default_with_internal()` entries become ARRS-compliant fallback data

### Out of Scope
- Registry server or hosting infrastructure
- User accounts, auth, API keys
- New toolchain providers beyond Python, Node, Bun, Go, Rust
- Version negotiation protocol (satisfied by existing semver resolver)

## Capabilities

### New Capabilities
- `arrs-spec`: Anvil Runtime Registry Specification — the formal TOML format describing runtime artifacts (name, version, platform, arch, URL, hash, mirrors, license)

### Modified Capabilities
- `hybrid-registry`: Existing `HybridRegistry` gains remote fetching. Resolution chain: local metadata cache → remote ARRS registry → embedded compiled-in fallback. Cache invalidation via TTL or explicit refresh.

## Approach

1. Define ARRS directory structure and all three file schemas (metadata.toml, sha256sums.txt, mirrors.json) in a spec document.
2. Add `RemoteRegistry` struct in `registry.rs`: HTTP GET to `{base_url}/registry/{runtime}/{version}/metadata.toml`, parse, cache to `.anvil/registry/`.
3. Modify `HybridRegistry::resolve()`: check local metadata cache first, then `RemoteRegistry`, then `default_with_internal()`.
4. Add `anvil registry refresh` CLI command (optional, in scope for tasks).
5. Add `reqwest` as optional dependency behind a `remote-registry` feature flag.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/registry.rs` | Modified | Add `RemoteRegistry`, extend `HybridRegistry` chain |
| `crates/anvil-core/src/resolver.rs` | None | Already uses `HybridRegistry`, continues unchanged |
| `crates/anvil-core/Cargo.toml` | Modified | Add `reqwest` behind `remote-registry` feature |
| `openspec/specs/hybrid-registry/spec.md` | Updated | Add remote fetch requirements |
| `openspec/specs/arrs-spec/spec.md` | New | ARRS format spec |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Registry URL unreachable | Medium | Local cache + embedded fallback guarantees offline operation |
| ARRS format changes upstream | Low | Spec is forge-owned open standard; versioned in semver |
| Cache staleness (wrong version resolved) | Low | TTL-based invalidation; `anvil registry refresh` forces update |

## Rollback Plan

- Feature-flag `remote-registry` (off by default) → disable restores old behavior
- Remove `RemoteRegistry` struct → `HybridRegistry` falls back to `default_with_internal()` as before
- ARRS spec is documentation-only; no rollback needed

## Dependencies

- `reqwest` crate (HTTP client, optional behind feature flag)
- ARRS static files hosted at a public URL (TBD — separate ops concern)

## Success Criteria

- [ ] `HybridRegistry` resolves a runtime from remote registry and caches it locally
- [ ] Offline with cache → resolves cached entry (no network)
- [ ] Offline without cache → falls back to embedded defaults
- [ ] `default_with_internal()` data is ARRS-compliant
- [ ] All existing resolver tests pass unchanged
