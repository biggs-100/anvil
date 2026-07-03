# Tasks: anvil-tui — Terminal Dashboard

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~700–900 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (setup + shell + dashboard) → PR 2 (remaining views + tests) |
| Delivery strategy | single-pr |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Project setup, App shell, Dashboard view | PR 1 | Base crate, terminal loop, dashboard widget |
| 2 | Runtimes, Diagnostics, History views + tests | PR 2 | Depends on PR 1; extends App with 3 views |

## Phase 1: Project Setup

- [x] 1.1 Add `"crates/anvil-tui"` to workspace `members` in root `Cargo.toml`
- [x] 1.2 Create `crates/anvil-tui/Cargo.toml` with ratatui, crossterm, tokio, anvil-core
- [x] 1.3 Add `anvil-tui` dep to `crates/anvil-cli/Cargo.toml`
- [x] 1.4 Add `Tui` variant to `Commands` enum + dispatch in `crates/anvil-cli/src/main.rs`

## Phase 2: App Shell

- [x] 2.1 Create `crates/anvil-tui/src/lib.rs` with `Tab` enum, `App` struct, data structs
- [x] 2.2 Implement terminal setup/teardown: crossterm raw mode, alternate screen, panic restore
- [x] 2.3 Implement event loop: crossterm poll, key dispatch, Tokio 5s interval for auto-refresh
- [x] 2.4 Implement tab switching (`1`–`4`), quit (`q`/Ctrl+C), scroll (`j`/`k`/arrows/Home/End)
- [x] 2.5 Handle `Event::Resize` — update stored size, re-render
- [x] 2.6 Expose `run_tui()` as public async fn with Engine, DiagnosticContext, health checks

## Phase 3: Dashboard View

- [x] 3.1 Implement `render_dashboard()`: health score colorized, runtime status, last operation, legend
- [x] 3.2 Implement `refresh()`: fetch `Engine::get_status()`, `DiagnosticEngine::score()`, `Engine::history()` limit 1

## Phase 4: Runtimes View

- [x] 4.1 Implement `render_runtimes()`: scrollable table with name/version/state/cache/shim
- [x] 4.2 Colorize states: Ready green, Synced blue, Broken red, others default

## Phase 5: Diagnostics View

- [x] 5.1 Implement `render_diagnostics()`: health score header + scrollable findings list
- [x] 5.2 Colorize severity: CRITICAL red, ERROR yellow, WARNING blue, INFO green; show quick fixes

## Phase 6: History View

- [x] 6.1 Implement `render_history()`: scrollable timeline sorted newest-first
- [x] 6.2 Colorize status: Success green, Failure red; cap at 50 entries

## Phase 7: Testing

- [x] 7.1 Unit tests for tab switching, key dispatch, scroll state via mock Engine
- [x] 7.2 Unit tests for data struct assembly from Engine responses
- [x] 7.3 Compile-check `anvil tui` subcommand wiring (`cargo check`)
