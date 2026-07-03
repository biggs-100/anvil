# Tasks: Rename Project from "forge" to "anvil"

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 1500–3000 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception accepted) |
| Delivery strategy | single-pr-default |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

## Phase 1: Core Crates

- [x] 1.1 Rename `crates/forge-core/` → `crates/anvil-core/`, update `Cargo.toml` name + internal refs
- [x] 1.2 Rename `crates/forge-drivers/` → `crates/anvil-drivers/`, update `Cargo.toml` + deps
- [x] 1.3 Rename `crates/forge-shim/` → `crates/anvil-shim/`, update `Cargo.toml` + refs

## Phase 2: CLI and TUI Crates

- [x] 2.1 Rename `crates/forge-cli/` → `crates/anvil-cli/`, update binary name + deps in `Cargo.toml`
- [x] 2.2 Rename `crates/forge-sdk/` → `crates/anvil-sdk/`, update `Cargo.toml` + deps
- [x] 2.3 Rename `crates/forge-tui/` → `crates/anvil-tui/`, update `Cargo.toml` + deps

## Phase 3: Config and Env

- [x] 3.1 Rename `forge.toml` → `anvil.toml`, update all code string refs
- [x] 3.2 Rename `forge.lock` → `anvil.lock`, update code refs
- [x] 3.3 Rename `forge.env` → `anvil.env`, update code refs
- [x] 3.4 Replace all `.forge/` → `.anvil/` references in source code
- [x] 3.5 Replace all `FORGE_*` env vars → `ANVIL_*` in source code
- [x] 3.6 Replace `https://registry.forge.sh` → `https://registry.anvil.dev`
- [x] 3.7 Replace `github.com/biggs-100/forge` → `github.com/biggs-100/anvil`
- [x] 3.8 Update root `Cargo.toml` workspace members, regenerate `Cargo.lock`

## Phase 4: Docs, Specs, SDKs, Extensions

- [x] 4.1 Rename `docs/specs/fcp-spec.md` → `acp-spec.md`, update FCP→ACP content
- [x] 4.2 Rename `docs/specs/fms-spec.md` → `ams-spec.md`, update FMS→AMS content
- [x] 4.3 Rename `openspec/specs/frrs-spec/` → `arrs-spec/`, update FRRS→ARRS content
- [x] 4.4 Update `sdks/go/` — package name, struct names, binary refs
- [x] 4.5 Update `sdks/python/` — dir rename, package name, class names
- [x] 4.6 Update `sdks/typescript/` — package name, class names, binary refs
- [x] 4.7 Update `extensions/vscode/` — all forge→anvil refs (exclude node_modules)
- [x] 4.8 Update `extensions/neovim/` — dir rename, module names, commands
- [x] 4.9 Update `README.md` and `AGENTS.md` — title, branding, CLI examples
- [x] 4.10 Update `openspec/` SDD artifacts referencing "forge" in content

## Phase 5: Verify

- [x] 5.1 Run `cargo build` — confirm zero errors
- [x] 5.2 Run `cargo test` — confirm all tests pass
- [x] 5.3 Run forge reference check — zero false positives in .rs and .toml files
- [x] 5.4 `anvil-cli` binary builds with correct name (`cargo build --release` passes)
