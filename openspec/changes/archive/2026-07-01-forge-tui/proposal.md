# Proposal: anvil-tui

## Intent

Replace ad-hoc CLI output with a terminal dashboard for `anvil tui`. Users need real-time visibility into anvil health, runtime status, diagnostics, and operation history — without leaving the terminal or parsing raw JSON.

## Scope

### In Scope
- New `crates/anvil-tui/` crate with Ratatui + crossterm
- Four views: Dashboard, Runtimes, Diagnostics, History
- Keyboard navigation (1-4 tabs, q/r, j/k/arrows, Home/End)
- Manual refresh (`r`) and optional 5s auto-refresh
- Wiring `anvil tui` subcommand into anvil-cli

### Out of Scope
- Mouse interaction or GUI — keyboard-only TUI
- Mutable state in TUI — reads from Engine facade
- Auto-refresh config persistence — initial implementation uses a compile-time default

## Capabilities

### New Capabilities
- `anvil-tui`: Terminal dashboard for Anvil — keyboard-driven Ratatui UI with four views reading from Engine facade

### Modified Capabilities
None — this is a new read-only interface over existing Engine APIs.

## Approach

New crate `crates/anvil-tui/` with three layers:
- **App**: terminal loop (crossterm event polling), tab state, refresh timer
- **Views**: four Ratatui `Widget` impls — Dashboard, Runtimes, Diagnostics, History
- **Data**: thin adapter fetching snapshots from `Engine` (health score via `DiagnosticEngine`, runtimes via `RuntimeManager`, history via `Engine::history()`)

No mutable state in TUI — each view reads a fresh snapshot on render. Tab `1`-`4` switch views, `r` triggers immediate refresh, `q` quits.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-tui/` | New | Ratatui dashboard crate |
| `crates/anvil-cli/` | Modified | Add `tui` subcommand |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Terminal size edge cases (tiny/no TTY) | Low | Fallback to error message if terminal too small |
| Refresh race on slow engines | Low | Async refresh with loading indicator |

## Rollback Plan

Remove `tui` subcommand from anvil-cli, delete `crates/anvil-tui/` from workspace, remove workspace dependency.

## Dependencies

- `ratatui` + `crossterm` on crates.io
- Existing `Engine` facade with `history()`, `DiagnosticEngine`, `RuntimeManager`

## Success Criteria

- [ ] `anvil tui` launches a Ratatui dashboard with four navigable views
- [ ] Tab switching, scroll, refresh, and quit all work
- [ ] Dashboard shows health score, runtime summary, last operation
- [ ] Diagnostics view shows severity-colored findings from DiagnosticEngine
- [ ] History view renders operation timeline with durations
