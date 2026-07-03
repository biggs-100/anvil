# Proposal: Rename Project from "anvil" to "anvil"

## Intent

Rename entire project from "anvil" to "anvil" due to severe SEO conflicts (Minecraft Anvil, AWS Anvil, GitHub Anvil, Foundry Anvil). "Anvil" maintains the metalworking/blacksmith theme while being uniquely searchable.

## Scope

### In Scope
- **6 Rust crates**: anvil-core→anvil-core, anvil-cli→anvil-cli, anvil-tui→anvil-tui, anvil-drivers→anvil-drivers, anvil-shim→anvil-shim, anvil-sdk→anvil-sdk
- **Binary**: rename executable to `anvil-cli`
- **Config files**: anvil.toml→anvil.toml, anvil.lock→anvil.lock, anvil.env→anvil.env, `.anvil/`→`.anvil/`
- **Env vars**: all `ANVIL_*` → `ANVIL_*`
- **GitHub refs**: `github.com/biggs-100/anvil` → `github.com/biggs-100/anvil`
- **Registry URL**: `https://registry.anvil.dev` → `https://registry.anvil.dev`
- **Public specs**: ACP→ACP, AMS→AMS, ARRS→ARRS
- **SDKs** (Go, Python, TS) and **extensions** (VS Code, Neovim) — internal references
- **All docs**: README, spec files, inline code docs

### Out of Scope
- GitHub repo URL migration (separate infra task)
- Package publication to crates.io / homebrew (separate task)
- Backward compatibility shims (full rename — no legacy support)
- CI/CD pipeline migration (handled in infra follow-up)

## Capabilities

### New Capabilities
None — this is a rename, no new behavioral capabilities.

### Modified Capabilities
None — no spec-level requirements change. All existing specs remain valid under the new name.

## Approach

Systematic find-and-replace across the entire workspace, crate by crate:

1. Rename crate dirs + update `Cargo.toml` names and internal deps
2. Rename config files and `.anvil/` dir to `.anvil/`
3. Update all `ANVIL_*` env var references in code and docs
4. Update public spec names (ACP→ACP, AMS→AMS, ARRS→ARRS)
5. Update URLs and GitHub refs in `Cargo.toml`, docs, and tooling config
6. Find-and-replace across SDKs and extensions
7. `cargo build` — zero anvil references in source
8. `cargo test` — all tests pass

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/*/Cargo.toml` | Modified | crate names + deps |
| `Cargo.toml` | Modified | workspace members |
| `anvil.toml`, `anvil.lock`, `anvil.env` | Renamed | → anvil.* |
| `.anvil/` | Renamed | → .anvil/ |
| `docs/` | Modified | all doc content |
| `openspec/specs/*/spec.md` | Modified | spec name refs |
| `sdks/*/` | Modified | internal anvil refs |
| `extensions/*/` | Modified | internal anvil refs |
| `README.md`, `AGENTS.md` | Modified | branding + refs |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Missed anvil string in source | Medium | `grep -r forge` before declaring done; CI build check |
| External consumers on old crate names | Low | publish under new names; old versions remain on crates.io |
| CI/CD refs still point to anvil | Low | handled in separate infra task; no code impact |

## Rollback Plan

Git revert. Every change is a file rename + content rename — fully reversible via `git revert` on the rename commit. Config file renames restore original names. No data migration involved.

## Dependencies

None. Self-contained rename within a single commit.

## Success Criteria

- [ ] `cargo build` passes with zero anvil references in Rust source code
- [ ] `cargo test` passes
- [ ] `anvil-cli --help` shows "anvil" everywhere (no "anvil" in output)
- [ ] anvil.toml / anvil.lock migrated to anvil.toml / anvil.lock
- [ ] No "anvil" strings remain in `crates/`, `sdks/`, `extensions/`, or `docs/` (excluding git history and external references)
