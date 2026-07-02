# Tasks: More Toolchains — LLVM/Clang + JDK

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 100–150 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr-default |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: LlvmProvider

- [x] 1.1 Add `LlvmProvider` struct + `RuntimeProvider` impl in `crates/forge-core/src/resolver.rs` (pattern-match `NodeProvider`)
- [x] 1.2 Add 5 default `RegistryEntry` rows for `llvm` in `crates/forge-core/src/registry.rs` `default_with_internal()` (windows/macos/linux × x86_64/aarch64)

## Phase 2: JdkProvider

- [x] 2.1 Add `JdkProvider` struct + `RuntimeProvider` impl in `crates/forge-core/src/resolver.rs` (pattern-match `NodeProvider`)
- [x] 2.2 Add 5 default `RegistryEntry` rows for `jdk` in `crates/forge-core/src/registry.rs` `default_with_internal()`

## Phase 3: Wiring

- [x] 3.1 Re-export `LlvmProvider`, `JdkProvider` from `crates/forge-core/src/lib.rs`
- [x] 3.2 Register both providers in `Resolver::new()` in `crates/forge-core/src/resolver.rs`

## Phase 4: Testing

- [x] 4.1 Unit test: `LlvmProvider.name()` returns `"llvm"`
- [x] 4.2 Unit test: `JdkProvider.name()` returns `"jdk"`
- [x] 4.3 Unit test: registry resolves `llvm` 18.1.0 for each platform/arch combo
- [x] 4.4 Unit test: registry resolves `jdk` 21.0.2 for each platform/arch combo
- [x] 4.5 Unit test: `default_with_internal()` contains `llvm` and `jdk` entries
- [x] 4.6 Unit test: resolve errors for nonexistent runtime return `Err`
