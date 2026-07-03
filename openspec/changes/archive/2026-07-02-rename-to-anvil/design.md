# Design: Rename Project from "anvil" to "anvil"

## Technical Approach

Systematic rename in dependency order: rename crates core→outward → fix cross-refs → config files → specs → docs/SDKs/extensions → env vars → verify. Single atomic commit. Per proposal — "renombrar todo" with no compat shims.

## Architecture Decisions

| Decision | Options | Choice | Rationale |
|----------|---------|--------|-----------|
| Crate rename order | Any order | Core first (anvil-core→anvil-core), then cli/drivers/shim/sdk/tui | Dependencies flow outward from core; renaming core first lets dependent crates update their path refs in one pass |
| Config file rename | Instant vs transitional | Instant rename | User chose "renombrar todo" — no compat shims, no legacy support |
| Binary name | `anvil` vs `anvil-cli` | `anvil-cli` | Per proposal scope; avoids global name collision with the `anvil` tool |
| Spec naming | ACP→ACP, AMS→AMS, ARRS→ARRS | Full rename | Per proposal; ARRS = Anvil Runtime Registry Specification |
| Env vars | `ANVIL_*` → `ANVIL_*` | Full rename | 71 occurrences across rust source; consistent find-and-replace |
| Data dir | `.anvil/` → `.anvil/` | Full rename | All code references: `home/.forge`, `workspace/.forge`, `.anvil/snapshots` |

## Data Flow

```
Cargo.toml workspace members
        │
        ▼
Core crate (anvil-core → anvil-core)    ← renamed first
  └─── lib.rs re-exports, Cargo.toml name/path deps
        │
        ▼
Dependent crates (anvil-cli, -tui, -sdk, -drivers, -shim)
  └─── Cargo.toml path deps, use statements, binary name
        │
        ▼
Config files (anvil.toml, .lock, .env, .anvil/)
  └─── Source code refs to "anvil.toml", ".anvil/", "ANVIL_*"
        │
        ▼
Docs & specs (fcp → acp, fms → ams, frrs → arrs)
SDKs (Go/Python/TS — package names, binary refs, class names)
Extensions (VS Code: commands, package.json; Neovim: .lua modules)
```

## File Changes

### Rust Crates (6 dir moves + content renames)

| Area | Action | Pattern |
|------|--------|---------|
| `crates/anvil-core/` → `crates/anvil-core/` | Move dir | Dir rename; `Cargo.toml`: `name = "anvil-core"` → `"anvil-core"`; all `pub use` unchanged |
| `crates/anvil-cli/` → `crates/anvil-cli/` | Move dir | `Cargo.toml`: name + deps (`anvil-core→anvil-core`, `anvil-drivers→anvil-drivers`, `anvil-tui→anvil-tui`); binary `name` in Cargo.toml |
| `crates/anvil-tui/` → `crates/anvil-tui/` | Move dir | `Cargo.toml`: name + dep (`anvil-core→anvil-core`) |
| `crates/anvil-drivers/` → `crates/anvil-drivers/` | Move dir | `Cargo.toml`: name |
| `crates/anvil-shim/` → `crates/anvil-shim/` | Move dir | `Cargo.toml`: name; binary ref in main.rs tests |
| `crates/anvil-sdk/` → `crates/anvil-sdk/` | Move dir | `Cargo.toml`: name + dep (`anvil-core→anvil-core`) |
| `Cargo.toml` (root) | Modify | workspace members: all paths updated |
| `Cargo.lock` | Regenerate | `cargo generate-lockfile` after all renames |

### Source Code — String Replacements

| Pattern | Files Affected | Scope |
|---------|---------------|-------|
| `anvil-core::` → `anvil-core::` | `anvil-cli/src/*.rs`, `anvil-tui/src/*.rs`, `anvil-sdk/src/*.rs` | All `use` and path refs in Rust source |
| `forge_tui::` → `anvil_tui::` | `anvil-cli/src/main.rs` (line 938) | Single call to TUI dispatch |
| `#[command(name = "anvil"` | `anvil-cli/src/main.rs` (line 31) | CLI metadata — name, version, about strings |
| `"anvil"` in help/about strings | `anvil-cli/src/main.rs` (~30 lines) | User-facing strings: `"anvil init"`, `"anvil.toml"`, etc. |
| `ANVIL_*` → `ANVIL_*` | 12 Rust files | Env vars: `ANVIL_PROFILE`, `ANVIL_HOME`, `ANVIL_REGISTRY_URL`, `ANVIL_MASTER_KEY`, `ANVIL_TRUSTED_KEYS`, `ANVIL_GPG_STRICT`, `ANVIL_VAR_`, `ANVIL_JOURNAL_PATH`, `ANVIL_PLUGIN_API_VERSION`, `ANVIL_BIN`, `ANVIL_SDK_*` |
| `https://registry.anvil.dev` → `https://registry.anvil.dev` | `anvil-core/src/lib.rs`, `anvil-core/src/gpg.rs`, `anvil-core/src/registry.rs`, `anvil-cli/src/main.rs` | Registry URL constants and defaults |
| `github.com/biggs-100/anvil` → `github.com/biggs-100/anvil` | `README.md`, `Cargo.toml` descriptions | No Cargo git dependencies found; README badges |

### Config & Data Files

| File | Action |
|------|--------|
| `anvil.toml` → `anvil.toml` | Rename file; update all source code string refs |
| `anvil.lock` → `anvil.lock` | Rename file; update all source code string refs |
| `anvil.env` → `anvil.env` | Rename file; update `find_forge_env()` call |
| `.anvil/` → `.anvil/` | Rename dir; refs in: `setup_shims()`, `uninstall_shims()`, `get_shim_binary_path()`, snapshot paths, cache paths, bundle paths |
| `anvil.local.toml`, `anvil.secrets` | Rename in docs only (AMS spec refs) |

### Docs, Specs & Config

| File | Action | Details |
|------|--------|---------|
| `docs/specs/acp-spec.md` → `docs/specs/acp-spec.md` | Rename + content | Header, inline "anvil" → "anvil", ACP→ACP |
| `docs/specs/ams-spec.md` → `docs/specs/ams-spec.md` | Rename + content | Header, inline "anvil" → "anvil", AMS→AMS |
| `docs/core-1.0-freeze.md` | Modify | ARRS→ARRS, forge→anvil |
| `docs/ecosystem.md` | Modify | ARRS→ARRS, forge→anvil |
| `docs/overview.md`, `docs/guide.md` | Modify | forge→anvil throughout |
| `openspec/config.yaml` | Modify | "anvil" in context string → "anvil" |
| `openspec/specs/arrs-spec/spec.md` | Rename to `arrs-spec/` + content | ARRS→ARRS references |
| `openspec/specs/acp-spec/`, `openspec/specs/ams-spec/` | Modify | Spec names in spec.md files |
| All other `openspec/` artifacts (~10 changes + rename-to-anvil) | Modify | "anvil" refs in design/spec/proposal content |
| `AGENTS.md` | Modify | Title, all "anvil" → "anvil" |
| `README.md` | Modify | Title, descriptions, CLI examples, crate table |
| `rust-toolchain.toml` | No change | Contains no anvil references |
| `.gitignore` | Review | If `.anvil/` listed, add `.anvil/` |

### SDKs

| SDK | Files | Changes |
|-----|-------|---------|
| **Go** (`sdks/go/`) | `client.go`, `client_test.go`, `types.go`, `go.mod` | Package name `forgesdk` → `anvilsdk`; `Forge` struct → `Anvil`; `NewForge()` → `NewAnvil()`; `ForgeError` → `AnvilError`; binary ref `"anvil"` → `"anvil-cli"` |
| **Python** (`sdks/python/`) | `forge_sdk/` dir → `anvil_sdk/`; `pyproject.toml`; `__init__.py`, `client.py`, `types.py`; tests | Dir rename; package `anvil-sdk` → `anvil-sdk`; `Forge` class → `Anvil`; `ForgeError` → `AnvilError`; binary ref |
| **TypeScript** (`sdks/typescript/`) | `package.json`; `src/`, `dist/`, `tests/` | Package `@forge/sdk` → `@anvil/sdk`; `Forge` class → `Anvil`; `ForgeError` → `AnvilError`; binary ref |
| **Go module path** | `go.mod` | `github.com/user/forge/sdk-go` → `github.com/user/anvil/sdk-go` |

### Extensions

| Extension | Files | Changes |
|-----------|-------|---------|
| **VS Code** (`extensions/vscode/`) | `package.json`, `src/*.ts`, `dist/*.js/.d.ts` | `forge-vscode` → `anvil-vscode`; all commands `anvil.*` → `anvil.*`; binary ref `"anvil"` → `"anvil-cli"`; all class/function names (`ForgePanel`, `ForgeStatusBar`, `ForgeError`) |
| **Neovim** (`extensions/neovim/`) | `lua/forge/` dir → `lua/anvil/`; all `.lua` files | Dir rename; module `anvil.*` → `anvil.*`; binary ref `"anvil"` → `"anvil-cli"`; `anvil.toml` → `anvil.toml`; user commands `ForgeStatus` → `AnvilStatus` etc. |

### Test Files — Specific Changes

| File | Env Var / String | New Value |
|------|-----------------|-----------|
| `anvil-core/tests/integration.rs` | `ANVIL_MASTER_KEY` | `ANVIL_MASTER_KEY` |
| `anvil-sdk/src/lib.rs` | `ANVIL_PROFILE`, `ANVIL_TEST_VAR` | `ANVIL_PROFILE`, `ANVIL_TEST_VAR` |
| `anvil-core/src/context/mod.rs` | `ANVIL_TEST_*` | `ANVIL_TEST_*` |
| `anvil-core/src/secrets/mod.rs` | `ANVIL_MASTER_KEY` | `ANVIL_MASTER_KEY` |
| `anvil-cli/tests/jsonrpc_test.rs` | `CARGO_BIN_EXE_ANVIL_CLI` | `CARGO_BIN_EXE_ANVIL_CLI` |
| `anvil-cli/tests/mcp_test.rs` | `CARGO_BIN_EXE_ANVIL_CLI` | `CARGO_BIN_EXE_ANVIL_CLI` |
| `sdks/typescript/tests/client.test.ts` | `ANVIL_BIN` | `ANVIL_BIN` |

### Order of Operations

1. Rename `crates/anvil-core/` → `crates/anvil-core/` + update its `Cargo.toml`
2. Rename remaining 5 crate dirs + update their `Cargo.toml` (names + dep paths)
3. Update root `Cargo.toml` workspace members
4. Bulk find-and-replace in Rust source: `anvil_core::` → `anvil_core::`, `forge_tui::` → `anvil_tui::`, `use forge::` → `use anvil::`
5. Bulk find-and-replace env vars `ANVIL_*` → `ANVIL_*`
6. Bulk find-and-replace URLs: `registry.anvil.dev` → `registry.anvil.dev`
7. Rename config files + `.anvil/` dir + update all string refs (`"anvil.toml"` → `"anvil.toml"`)
8. Rename spec docs + update openspec content
9. Rename SDK dirs + bulk rename content (packages, classes, binary name)
10. Rename extension dirs + bulk rename content
11. Update README, AGENTS.md, docs/*.md
12. `cargo generate-lockfile`
13. `cargo build` + `cargo test`

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Build | All crates compile | `cargo build` after every rename step |
| Unit | Individual crate tests | `cargo test -p anvil-core -p anvil-cli ...` |
| Integration | Full workspace | `cargo test` — all env var renames must match |
| SDK | Go/Python/TS smoke | `go test ./...`, `pytest`, `npm test` (skip if no runtime) |
| Name sweep | Zero anvil refs in source | `rg -i "anvil" --crates/ --sdks/ --extensions/ --docs/ --openspec/ -g '!target/' -g '!node_modules/'` |
| CLI output | `anvil-cli --help` | Verify no "anvil" in output text |

Verification gate: `rg -i "anvil" crates/ sdks/ extensions/ docs/ --type rust --type add --type py --type ts` returns zero hits (excluding `.git/`, `target/`, `node_modules/`).

## Migration / Rollout

No migration required. Single atomic commit. Rollback = `git revert`.

## Interfaces / Contracts

None affected. All existing public API types (`Engine`, `Operation`, `ForgeContext`, etc.) keep their Rust type names. The SDK client classes rename (`Forge` → `Anvil`) — this is a breaking change for external SDK consumers, handled via new package publication.

## Open Questions

None — all scope decisions made in proposal.
