# Proposal: Modularize Forge Core Runtime Engine

## Intent

Decompose the monolithic `crates/forge-core/src/lib.rs` (~1300 lines) into 8 domain-specific submodules to establish a decoupled, sustainable, and scalable architecture.

## Scope

### In Scope
- Create `types.rs`: Common domain types (`RuntimeId`, `RuntimeVersion`, `Platform`, `Architecture`, `Hash`).
- Refactor `manifest.rs`: `forge.toml` path/load/validation logic.
- Refactor `resolver.rs`: `RuntimeProvider` abstractions (Node, Python).
- Refactor `installer.rs`: Download, extraction, Zip Slip protection.
- Refactor `registry.rs`: HybridRegistry resolution & metadata checks.
- Refactor `cache.rs`: Cached toolchains and shim caches.
- Refactor `environment.rs`: PATH manipulation, parsing, and masking.
- Refactor `launcher.rs`: Process spawning and signal forwarding.
- Restructure `lib.rs`: Expose new submodules, re-export stable API types.
- Test Relocation: Inline unit tests inside submodules; integration tests to `tests/`.

### Out of Scope
- Command additions: `forge sync`, `forge gc`, `forge clean` (Phase 5).
- Secret management CLI/commands: `forge secret` (Phase 6).

## Capabilities

### New Capabilities
- `runtime-engine-types`: Holds common domain types.
- `runtime-engine-manifest`: Locates, loads, and represents manifests.
- `runtime-engine-resolver`: Modular resolver and RuntimeProvider interfaces.
- `runtime-engine-installer`: Trait-based downloader and extractors.
- `runtime-engine-registry`: Coordinates local/cached registries.
- `runtime-engine-cache`: Manages cached toolchains and shims caches.
- `runtime-engine-environment`: Computes PATH and environment mappings.
- `runtime-engine-launcher`: Spawns and forwards processes cleanly.

### Modified Capabilities
- None

## Approach

1. Decompose `lib.rs` into new module files, resolving circular dependencies by moving shared types to `types.rs`.
2. Re-export public APIs in `lib.rs` to maintain compatibility with `forge-cli` and downstream crates.
3. Migrate and organize unit tests into target submodules and construct verification integration tests.
4. Run `cargo test` and `cargo check` after each module extraction to guarantee regression-free incremental refactoring.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/lib.rs` | Modified | Monolith split; exposes submodules and re-exports stable API. |
| `crates/forge-core/src/types.rs` | New | Shared domain types. |
| `crates/forge-core/src/manifest.rs` | New | Manifest load/parsing. |
| `crates/forge-core/src/resolver.rs` | New | Provider interfaces and resolver logic. |
| `crates/forge-core/src/installer.rs` | New | Downloader and extractors. |
| `crates/forge-core/src/registry.rs` | New | Registry coordinates resolution. |
| `crates/forge-core/src/cache.rs` | New | Toolchain and shim cache management. |
| `crates/forge-core/src/environment.rs` | New | Env parsing/masking and PATH calculation. |
| `crates/forge-core/src/launcher.rs` | New | Process execution & signal forwarding. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Circular dependencies during extraction | Med | Early isolation of primitives in `types.rs`. |
| API breakage for CLI | Low | Expose stable interface facade in `lib.rs`. |
| Test regressions | Low | Continuous test execution (`cargo test`). |

## Rollback Plan

Revert all changes using git:
```bash
git checkout -- crates/forge-core/
rm -f crates/forge-core/src/{types,manifest,resolver,installer,registry,cache,environment,launcher}.rs
```

## Dependencies

- None (Standard library and existing workspace dependencies only).

## Success Criteria

- [ ] All unit and integration tests compile and pass via `cargo test`.
- [ ] No circular dependencies or compiler errors present.
- [ ] Monolith `lib.rs` size reduced by >= 80%.
- [ ] Stable public API remains backwards-compatible.
