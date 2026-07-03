# Proposal: More Toolchains — LLVM/Clang + JDK

## Intent

Add LLVM/Clang and JDK as managed runtimes alongside existing Node/Python/Bun/Go/Rust. Users need C/C++ compilation (LLVM) and Java/Kotlin/Scala/Groovy compilation (JDK) without manual toolchain management.

## Scope

### In Scope
- `LlvmProvider` and `JdkProvider` implementing `RuntimeProvider` trait
- ARRS default registry entries for both (Windows/MacOS/Linux × x86_64/aarch64)
- `anvil.toml` validation — `llvm` and `jdk` accepted in `[runtimes]`
- Unit tests for both providers

### Out of Scope
- Android SDK, CUDA, .NET, Flutter, or other runtimes
- Building LLVM/JDK from source
- Runtime-specific shim generation (follows existing generic shim path)

## Capabilities

### New Capabilities
- `runtime-llvm`: LLVM/Clang toolchain provider — downloads pre-built releases from `github.com/llvm/llvm-project/releases`
- `runtime-jdk`: JDK toolchain provider — downloads from Adoptium API with LTS+current version support

### Modified Capabilities
- `runtime-providers`: Two new built-in providers (`llvm`, `jdk`) added to the resolver alongside the existing five

## Approach

Follow the exact same pattern as existing providers (NodeProvider, PythonProvider, etc.) — each provider struct implements `RuntimeProvider` with `name()` and `resolve()` delegating to `resolve_from_registry()`. Registry entries follow the ARRS format with default URLs and placeholder hashes. No new external dependencies.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/resolver.rs` | Modified | Add `LlvmProvider`, `JdkProvider`, register in `Resolver::new()` |
| `crates/anvil-core/src/registry.rs` | Modified | Add default `RegistryEntry` rows for both runtimes |
| `crates/anvil-core/src/manifest.rs` | Modified | Add `llvm`, `jdk` to accepted runtime keys |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| LLVM/JDK download URLs change | Medium | ARRS pattern — only registry URL updates, no code changes |
| ARM64 builds missing for some versions | Low | Existing emulation fallback path handles this |

## Rollback Plan

Revert the three files above. Remove `llvm` and `jdk` entries from `Resolver::new()`. Existing runtimes are unaffected.

## Dependencies

- LLVM pre-built releases at `github.com/llvm/llvm-project/releases`
- Adoptium API at `api.adoptium.net/v3/binary/latest/<version>/ga`

## Success Criteria

- [ ] `LlvmProvider` resolves LLVM 18.1.0 across all 3 platforms
- [ ] `JdkProvider` resolves JDK 21.0.2 across all 3 platforms
- [ ] Existing five runtimes continue resolving identically
- [ ] `cargo test` passes with no regressions
