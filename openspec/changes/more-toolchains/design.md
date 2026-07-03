# Design: More Toolchains — LLVM/Clang + JDK

## Technical Approach

Follow the exact `RuntimeProvider` trait pattern from existing providers (Node, Python, Bun, Go, Rust). Each new provider is a unit struct that delegates `resolve()` to `resolve_from_registry()`, which resolves against the 4-tier registry chain (flat entries → ARRS cache → ARM64 fallback → embedded defaults). All artifact metadata — download URLs, SHA-256 hashes, sizes — lives in registry entries, not in provider code. Providers are registered in `Resolver::new()` by name and re-exported from `crate::lib`.

## Architecture Decisions

### Decision: Registry-driven resolution (no direct HTTP in providers)

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Provider constructs URLs itself | Ties code to source layout; URL changes require code change | REJECTED |
| Registry entries carry all URL/size/hash metadata | URL changes are registry-only (no code change); works with remote ARRS | SELECTED |
| Provider fetches checksum from release API | Adds HTTP retry logic per provider; fragile | REJECTED |

**Rationale**: The existing `resolve_from_registry()` + `HybridRegistry::resolve()` chain already handles the 4-tier fallback (flat → ARRS cache → ARM64 fallback → embedded defaults). This means LLVM and JDK work identically to existing runtimes: the provider just names itself, and the registry does all URL matching, emulation fallback, and checksum bookkeeping. No new dependencies, no HTTP in providers.

### Decision: LLVM download source — GitHub releases

**Choice**: `https://github.com/llvm/llvm-project/releases/download/llvmorg-{version}/{asset}`  
**Alternatives considered**: LLVM's own apt/homebrew packages (not cross-platform), building from source (too slow)  
**Rationale**: GitHub releases are pre-built, cross-platform, and follow a predictable asset naming convention. Same pattern as Bun and Go providers already use.

### Decision: JDK download source — Adoptium API v3

**Choice**: `https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse`  
**Alternatives considered**: Oracle JDK (licensing), OpenJDK builds (less standardized), Zulu API  
**Rationale**: Adoptium (Eclipse Temurin) is the de-facto standard open-source JDK build, LTS-focused, with reliable binary API. The v3 endpoint returns a redirect to the actual download URL with checksum headers.

### Decision: No manifest.rs changes needed

**Rationale**: `ForgeConfig.runtimes` is `HashMap<String, String>` — there is no explicit allowlist of runtime keys. `llvm` and `jdk` are accepted by existing parsing without changes. The proposal's manifest.rs entry is removed from the plan.

## Data Flow

```
anvil.toml                     Resolver::new()                HybridRegistry
┌──────────────┐              ┌────────────────────┐        ┌──────────────────┐
│ [runtimes]    │  key lookup │ providers["llvm"]  │  call  │ resolve("llvm",  │
│ llvm = 18.1.0 │──────────→  │   → LlvmProvider   │───────→│   "18.1.0",      │
│ jdk = 21.0.2  │             │ providers["jdk"]   │        │   platform, arch)│
└──────────────┘              │   → JdkProvider    │        └────────┬─────────┘
                              └────────────────────┘                 │
                                                   Tier 4 fallback   │
                                              ┌──────────────────────┘
                                              ▼
                              RegistryEntry { url, sha256, size }
                              ───────────────────────────────────────→ RuntimeLock
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/src/resolver.rs` | Modify | Add `LlvmProvider`, `JdkProvider` structs with `RuntimeProvider` impl; register in `Resolver::new()` |
| `crates/anvil-core/src/registry.rs` | Modify | Add `llvm` and `jdk` default `RegistryEntry` rows to `default_with_internal()` (all 5 platform/arch combos each) |
| `crates/anvil-core/src/lib.rs` | Modify | Add `LlvmProvider`, `JdkProvider` to re-exports |

## Interfaces / Contracts

### New structs (same pattern as existing providers)

```rust
pub struct LlvmProvider;
impl RuntimeProvider for LlvmProvider {
    fn name(&self) -> &str { "llvm" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}

pub struct JdkProvider;
impl RuntimeProvider for JdkProvider {
    fn name(&self) -> &str { "jdk" }
    fn resolve(&self, version_req: &str, platform: &str, arch: &str, registry: &HybridRegistry) -> Result<RuntimeLock, String> {
        resolve_from_registry(self.name(), version_req, platform, arch, registry)
    }
}
```

### Default registry entries — URL patterns

**LLVM** (`github.com/llvm/llvm-project/releases/download/llvmorg-{ver}/`):

| Platform | Arch | Asset pattern |
|----------|------|---------------|
| windows | x86_64 | `LLVM-{ver}-win64.zip` or `clang+llvm-{ver}-x86_64-pc-windows-msvc.tar.xz` |
| macos | x86_64 | `clang+llvm-{ver}-x86_64-apple-darwin.tar.xz` |
| macos | aarch64 | `clang+llvm-{ver}-arm64-apple-darwin.tar.xz` |
| linux | x86_64 | `clang+llvm-{ver}-x86_64-linux-gnu.tar.xz` |
| linux | aarch64 | `clang+llvm-{ver}-aarch64-linux-gnu.tar.xz` |

**JDK** (`api.adoptium.net/v3/binary/latest/{major}/ga/`):

| Platform | Arch | API path suffix |
|----------|------|-----------------|
| windows | x86_64 | `windows/x64/jdk/hotspot/normal/eclipse` |
| macos | x86_64 | `mac/x64/jdk/hotspot/normal/eclipse` |
| macos | aarch64 | `mac/aarch64/jdk/hotspot/normal/eclipse` |
| linux | x86_64 | `linux/x64/jdk/hotspot/normal/eclipse` |
| linux | aarch64 | `linux/aarch64/jdk/hotspot/normal/eclipse` |

### Shim names

| Runtime | Shim binaries |
|---------|--------------|
| llvm | `clang`, `clang++`, `clangd`, `lld` |
| jdk | `java`, `javac`, `jar` |

Shim generation follows the existing generic path (scan bin dirs, no per-provider shim logic needed).

## Extraction notes

- LLVM archives (`.tar.xz`) extract into a directory like `clang+llvm-18.1.0-x86_64-linux-gnu/`. The `bin/` directory is one level deeper than Node/Python archives. The existing `TarXzExtractor` handles the format; shim scanning (`find_bin_dirs`) already recurses into subdirectories, so no custom extraction logic is needed.
- JDK archives (`.tar.gz` or `.zip`) extract into a directory like `jdk-21.0.2+13/Contents/Home/bin/` on macOS or `jdk-21.0.2+13/bin/` on Linux. The nested `Contents/Home/` on macOS is handled by the existing recursive bin scanning.
- **No custom `Extractor` configuration is required** — existing extractors and shim scanning handle the structure.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Provider names | `assert_eq!(LlvmProvider.name(), "llvm")` |
| Unit | Provider registration | Register both in `Resolver`, verify `resolve()` delegates to registry |
| Unit | Registry resolution | Add test entries for llvm/jdk to a `HybridRegistry`, verify `resolve()` returns correct entries |
| Unit | ARM64 Windows fallback | Verify Windows aarch64 falls back to x86_64 for both new runtimes |
| Integration | Default entries | Verify `default_with_internal()` contains llvm and jdk entries |

## Migration / Rollout

No migration required. New providers are additive — existing runtimes are unaffected. Users with `[runtimes] llvm = "..."` or `jdk = "..."` in their `anvil.toml` will automatically resolve on the next `anvil install` / `anvil update`.

## Open Questions

- [ ] LLVM Windows asset format: verify whether recent releases ship `.tar.xz` or only `.exe` installer. If only `.exe`, Windows LLVM support may need 7-zip extraction or alternative source (e.g., LLVM's own Windows builds via GitHub or llvm-windows releases).
- [ ] JDK API version mapping: Adoptium API major version is feature number (e.g., `21`). Verify semver `"21.0.2"` maps to API path `latest/21/ga/` correctly and the returned binary version string matches the resolved semver.
