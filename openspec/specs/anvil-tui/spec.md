# anvil-tui Specification

## Purpose

Terminal dashboard for Anvil — keyboard-driven Ratatui UI with four read-only views (Dashboard, Runtimes, Diagnostics, History). Shows real-time anvil health, runtimes, diagnostic findings, and operation history without mutating engine state.

## Requirements

### Requirement: Dashboard View

The Dashboard MUST show health score (0–100 colorized), runtime count (total/ready/broken from `Engine::status()`), last operation result, and keyboard legend. Health score MUST come from `DiagnosticEngine::score()`. SHOULD refresh on `r`.

#### Scenario: Full state rendered

- GIVEN health score 85, 3 runtimes (2 ready, 1 broken), and a successful last operation
- WHEN the Dashboard view opens
- THEN it shows score 85, "3 (2 ready, 1 broken)", and the last operation result
- AND the keyboard legend is visible

#### Scenario: Auto-refresh after 5s

- GIVEN the Dashboard has been open for 5 seconds without manual refresh
- WHEN the auto-refresh timer fires
- THEN engine state is re-queried and the display updates

### Requirement: Runtimes View

The Runtimes view MUST show all runtimes from anvil.toml with name, version, state, lock state, cache path, and shim status. SHOULD colorize by state. MUST scroll with `j`/`k` and arrows.

#### Scenario: Three runtimes listed

- GIVEN three runtimes configured in anvil.toml
- WHEN user opens Runtimes view
- THEN all three display with complete metadata

#### Scenario: No runtimes configured

- GIVEN anvil.toml defines zero runtimes
- WHEN user opens Runtimes view
- THEN an empty state message is shown (no panic)

### Requirement: Diagnostics View

MUST run `DiagnosticEngine` on load. Each finding MUST show code, severity, confidence, message. Severity colors: CRITICAL red, ERROR yellow, WARNING blue, INFO green. SHOULD show quick fix suggestions. MUST scroll.

#### Scenario: Findings with severity colors

- GIVEN 4 findings (CRITICAL, ERROR, WARNING, INFO)
- WHEN user opens Diagnostics view
- THEN all 4 render with correct severity colors and full detail

#### Scenario: No findings

- GIVEN DiagnosticEngine returns zero findings
- WHEN user opens Diagnostics view
- THEN "All healthy" empty state is shown

### Requirement: History View

MUST show `Engine::history()` results sorted most recent first. Each entry MUST show ID, runtime, duration, status. Status colors: Success green, Failure red. SHOULD limit to 50 entries. MUST scroll.

#### Scenario: Mixed status history

- GIVEN Engine history has 10 operations with mixed success/failure
- WHEN user opens History view
- THEN all 10 entries display sorted newest-first with correct status colors

#### Scenario: Overflow capped

- GIVEN 75 operations in Engine history
- WHEN user opens History view
- THEN only the 50 most recent entries are shown

### Requirement: Navigation and Controls

MUST switch tabs via `1`–`4` or left/right arrows. MUST quit on `q` or Ctrl+C. MUST scroll with `j`/`k`, arrows, Home/End. MUST handle terminal resize. MUST exit cleanly restoring terminal state. SHOULD show keyboard legend.

#### Scenario: Tab switching

- GIVEN user is on Dashboard (Tab 1)
- WHEN user presses `3`
- THEN view switches to Diagnostics

#### Scenario: Clean quit

- GIVEN anvil-tui is running
- WHEN user presses `q`
- THEN app exits and terminal is restored to original state

#### Scenario: Terminal resize

- GIVEN anvil-tui is running at 120×40
- WHEN terminal resizes to 80×24
- THEN UI re-renders to fit new dimensions without errors

### Requirement: Error Handling

MUST handle engine errors inline — display error message, never panic. MUST show error state if no anvil.toml found. MUST NOT panic on empty data in any view.

#### Scenario: Missing anvil.toml

- GIVEN no anvil.toml exists in the current directory
- WHEN user launches anvil-tui
- THEN "No anvil.toml found" error message is shown in the dashboard

#### Scenario: Engine failure

- GIVEN `DiagnosticEngine::score()` returns an error
- WHEN Dashboard view renders
- THEN health score area shows an error indicator instead of crashing
