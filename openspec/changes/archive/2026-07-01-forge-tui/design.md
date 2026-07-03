# Design: anvil-tui — Terminal Dashboard

## Technical Approach

New `crates/anvil-tui/` crate using Ratatui + crossterm. Single `App` struct owns all state, runs a crossterm event loop with a Tokio 5s interval for auto-refresh. Four tab views (Dashboard, Runtimes, Diagnostics, History) rendered via `match` on active tab index. Data fetched synchronously from Engine facade on each render — no caching, read-only. Async Engine calls run via `tokio::task::block_in_place` or the existing Tokio runtime (anvil-cli is `#[tokio::main]`). Events dispatched by `match` on key code. Terminal resize handled by crossterm's `Event::Resize`.

## Architecture Decisions

### Decision: Single `App` struct with enum tab state
| Option | Tradeoff | Decision |
|--------|----------|----------|
| Single App struct, `active_tab: Tab` enum | Simple, direct; one rendering switch | **Chosen** — 4 views, no shared state complexity |
| Separate component per tab | Flexible but adds trait indirection | Rejected — premature abstraction for 4 views |

### Decision: Synchronous Data Fetch on render
| Option | Tradeoff | Decision |
|--------|----------|----------|
| Block on Engine in rendering path | Simplest, no channels; works inside existing Tokio runtime | **Chosen** — Engine calls are fast (cache reads), anvil-core frozen surface |
| Async with channel back to TUI | Handles slow Engine calls but adds channel wiring | Rejected — perf not justified for local cache reads |

### Decision: Blocked fetch using `block_in_place`
| Option | Tradeoff | Decision |
|--------|----------|----------|
| Tokio `block_in_place` + `Handle::block_on` | Matches existing anvil-cli pattern in `run_diagnostic_engine` | **Chosen** — reuse proven pattern |
| `std::thread::spawn` + channel | Avoids Tokio runtime coupling | Rejected — anvil-cli already runs Tokio |

### Decision: Crossterm raw mode + event polling
| Option | Tradeoff | Decision |
|--------|----------|----------|
| Crossterm event polling | Handles resize, keyboard; no extra deps | **Chosen** — crossterm is already the Ratatui backend |
| `mio`/`signal` for resize | More complex, no benefit | Rejected |

### Decision: Auto-refresh via Tokio interval in event loop
| Option | Tradeoff | Decision |
|--------|----------|----------|
| `tokio::time::interval` in event select | Natural fit with existing Tokio runtime | **Chosen** — anvil-cli is `#[tokio::main]` |
| Crossterm tick timer | Works outside Tokio | Rejected — doubles timer logic |

## Data Flow

```
 User Keyboard ──→ Crossterm poll ──→ App::handle_event()
                                           │
                              ┌────────────┼────────────┐
                              │ r/5s timer  │ 1-4/q/j/k  │
                              ▼             │             │
                        App::refresh()       │        App state update
                              │              │              │
                    ┌─────────┼─────────┐    │              │
                    ▼         ▼         ▼    │              │
              Engine::   Diagnostic   load_  │              │
              history()  Engine::run() config│              │
                    │         │              │              │
                    ▼         ▼              ▼              │
               App data    App data      App data           │
                    └─────────┼──────────────┘              │
                              ▼                             ▼
                         App::render() ──→ Ratatui Terminal::draw()
                              │
                    ┌─────────┼──────────┐
                    ▼         ▼          ▼
              Dashboard  Runtimes   Diagnostics  History
                Widget    Widget      Widget      Widget
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `"crates/anvil-tui"` to workspace members |
| `crates/anvil-cli/Cargo.toml` | Modify | Add `anvil-tui` dependency |
| `crates/anvil-cli/src/main.rs` | Modify | Add `Tui` variant to `Commands` enum, dispatch in `run_cli` |
| `crates/anvil-tui/Cargo.toml` | Create | Dependencies: anvil-core, ratatui, crossterm, tokio |
| `crates/anvil-tui/src/lib.rs` | Create | `App` struct, event loop, tab rendering, data fetching |

## Interfaces / Contracts

### `App` struct

```rust
pub struct App {
    active_tab: Tab,         // Dashboard=0, Runtimes=1, Diagnostics=2, History=3
    scroll_offset: usize,
    dashboard_data: DashboardData,
    runtimes_data: RuntimesData,
    diagnostics_data: DiagnosticsData,
    history_data: HistoryData,
    engine: Engine,
    diag_ctx: DiagnosticContext,
    plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
    should_quit: bool,
    terminal_size: (u16, u16), // (cols, rows)
}

enum Tab {
    Dashboard,
    Runtimes,
    Diagnostics,
    History,
}

struct DashboardData {
    health_score: u8,
    lifecycle_state: String,
    last_operation: Option<OperationSummary>,
}

struct RuntimesData {
    runtimes: Vec<RuntimeEntry>,
    error: Option<String>,
}

struct RuntimeEntry {
    name: String,
    version: String,
    state: String,
    lock_state: String,
    cache_path: Option<String>,
    shim_present: bool,
}

struct DiagnosticsData {
    score: u8,
    findings: Vec<DiagnosticReport>,
    error: Option<String>,
}

struct HistoryData {
    entries: Vec<OperationSummary>,
    error: Option<String>,
}
```

### Event loop signature (inside `run_cli` dispatch)

```rust
pub async fn run_tui(
    engine: Engine,
    diag_ctx: DiagnosticContext,
    plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
) -> Result<(), String>
```

### Tab rendering

```rust
impl App {
    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area());
        // Tab bar at bottom
        // Active tab content in main area
        match self.active_tab {
            Tab::Dashboard => self.render_dashboard(frame, layout[0]),
            Tab::Runtimes => self.render_runtimes(frame, layout[0]),
            Tab::Diagnostics => self.render_diagnostics(frame, layout[0]),
            Tab::History => self.render_history(frame, layout[0]),
        }
    }
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Tab switching, key dispatch, scroll state | Mock `Engine`, direct `App::handle_event()` calls |
| Unit | Data struct assembly from Engine responses | Pure function tests for data mappers |
| Integration | `anvil tui` subcommand wiring | `cargo check` that enum variant and dispatch compile |
| E2E | Full TUI launch (smoke test) | Manual: launch `anvil tui`, verify 4 tabs, quit with `q` |

## Migration / Rollout

No migration required. Additive change — new subcommand, new crate. Rollback: delete `anvil-tui` from workspace and CLI.

## Open Questions

- None — design decisions resolved in spec.
