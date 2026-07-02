Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Forge Bootstrap

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 600-800 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Workspace & Configuration | PR 1 | Cargo workspace setup, manifest, env parsing |
| 2 | Runtime Downloader & Cache | PR 2 | Tokio downloader, SHA-256, extraction |
| 3 | Spawning & System Drivers | PR 3 | Subprocess wrapper, system drivers |
| 4 | CLI commands & AI diagnostics | PR 4 | Clap parser, context/doctor diagnostics |

## Phase 1: Workspace & Configuration (PR 1)

- [x] 1.1 Create workspace root `Cargo.toml` and crates: `crates/forge-cli`, `crates/forge-core`, `crates/forge-drivers`. Verify workspace compile via `cargo check`.
- [x] 1.2 Define `forge.toml` manifest structs with `serde` / `toml` in `crates/forge-core/src/lib.rs`. Test tool version parser.
- [x] 1.3 Implement `forge.env` parser in `crates/forge-core/src/lib.rs`. Verify masking helper function on secret credentials.

## Phase 2: Runtime Downloader & Cache (PR 2)

- [x] 2.1 Set up global cache path (`~/.forge/runtimes/`) and lockfile generator structs (`forge.lock`) in `crates/forge-core/src/lib.rs`. Test lockfile serializer.
- [x] 2.2 Implement concurrent downloader using `tokio` and `reqwest` in `crates/forge-core/src/lib.rs`. Verify file deletion on SHA-256 mismatch.
- [x] 2.3 Implement zip/tar.gz extraction in `crates/forge-core/src/lib.rs` (using `zip` and `tar`). Verify target extraction structure.

## Phase 3: Spawning & System Drivers (PR 3)

- [x] 3.1 Implement environment activation and path prepending engine in `crates/forge-core/src/lib.rs`. Test priority ordering.
- [x] 3.2 Implement subprocess spawning wrapped runner for `run` / `shell` in `crates/forge-core/src/lib.rs`. Verify host environment isolation.
- [x] 3.3 Implement `winget`/`brew`/`apt`/`pacman` execution wrappers in `crates/forge-drivers/src/lib.rs`. Test driver exit code propagation.

## Phase 4: CLI Interface & AI Diagnostics (PR 4)

- [x] 4.1 Implement command CLI flags with Clap in `crates/forge-cli/src/main.rs`. Test parser with `run`, `shell`, `ai` subcommands.
- [x] 4.2 Implement `forge ai context` command displaying JSON map with masked secrets. Verify redaction formats.
- [x] 4.3 Implement `forge ai doctor` diagnostic checks command with severity and remediation instructions. Verify output schema.
