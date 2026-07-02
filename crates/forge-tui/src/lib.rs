//! forge-tui — Terminal dashboard for Forge environments.
//!
//! Ratatui-based keyboard-driven TUI with four read-only views:
//! Dashboard, Runtimes, Diagnostics, and History.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table,
    },
    Frame, Terminal,
};
use tokio::runtime::Handle;

use forge_core::{
    api::v1::OperationSummary, find_forge_toml, load_config, load_lockfile, DiagnosticContext,
    DiagnosticEngine, DiagnosticMode, Engine, Finding, HealthCheck, Severity,
};

// ---------------------------------------------------------------------------
// Tab
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard = 0,
    Runtimes = 1,
    Diagnostics = 2,
    History = 3,
}

impl Tab {
    fn from_index(i: usize) -> Self {
        match i {
            0 => Tab::Dashboard,
            1 => Tab::Runtimes,
            2 => Tab::Diagnostics,
            3 => Tab::History,
            _ => Tab::Dashboard,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Tab::Dashboard => "1  Dashboard",
            Tab::Runtimes => "2  Runtimes",
            Tab::Diagnostics => "3  Diagnostics",
            Tab::History => "4  History",
        }
    }
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct DashboardData {
    health_score: u8,
    lifecycle_state: String,
    runtime_count: usize,
    ready_runtimes: usize,
    broken_runtimes: usize,
    last_operation: Option<OperationSummary>,
    config_error: Option<String>,
}

#[derive(Debug, Clone)]
struct RuntimeEntry {
    name: String,
    version: String,
    state: String,
    #[allow(dead_code)]
    lock_state: String,
    cache_path: Option<String>,
    shim_present: bool,
}

#[derive(Debug, Default)]
struct RuntimesData {
    runtimes: Vec<RuntimeEntry>,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct DiagnosticsData {
    score: u8,
    findings: Vec<Finding>,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct HistoryData {
    entries: Vec<OperationSummary>,
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    active_tab: Tab,
    scroll_positions: [usize; 4],
    should_quit: bool,
    refresh_requested: bool,
    engine: Engine,
    diag_ctx: DiagnosticContext,
    plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
    dashboard_data: DashboardData,
    runtimes_data: RuntimesData,
    diagnostics_data: DiagnosticsData,
    history_data: HistoryData,
    last_refresh: Instant,
    cols: u16,
    rows: u16,
}

impl App {
    /// Create a new App with the given engine, diagnostic context, and health checks.
    pub fn new(
        engine: Engine,
        diag_ctx: DiagnosticContext,
        plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
    ) -> Self {
        Self {
            active_tab: Tab::Dashboard,
            scroll_positions: [0; 4],
            should_quit: false,
            refresh_requested: false,
            engine,
            diag_ctx,
            plugin_health_checks,
            dashboard_data: DashboardData::default(),
            runtimes_data: RuntimesData::default(),
            diagnostics_data: DiagnosticsData::default(),
            history_data: HistoryData::default(),
            last_refresh: Instant::now(),
            cols: 0,
            rows: 0,
        }
    }

    /// Run the TUI application.
    ///
    /// Sets up the terminal, runs the event loop, and restores the terminal on exit.
    pub async fn run(
        engine: Engine,
        diag_ctx: DiagnosticContext,
        plugin_health_checks: Vec<Arc<dyn HealthCheck>>,
    ) -> Result<(), String> {
        enable_raw_mode().map_err(|e| format!("{e}"))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| format!("{e}"))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal =
            Terminal::new(backend).map_err(|e| format!("enable to initialise terminal: {e}"))?;

        let mut app = Self::new(engine, diag_ctx, plugin_health_checks);

        // Set up panic hook to restore terminal on crash
        let orig_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
            let _ = stdout.flush();
            orig_hook(panic_info);
        }));

        let result = app.run_loop(&mut terminal).await;

        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        // Restore original panic hook
        let _ = std::panic::take_hook();

        result
    }

    /// The core event loop.
    async fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), String> {
        // Initial data fetch
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                self.refresh_all_data().await;
            })
        });

        let refresh_interval = Duration::from_secs(5);
        let poll_timeout = Duration::from_millis(100);

        loop {
            // Render current state
            terminal
                .draw(|f| self.render(f))
                .map_err(|e| format!("render error: {e}"))?;

            if self.should_quit {
                break;
            }

            // Poll for keyboard events
            if event::poll(poll_timeout).map_err(|e| format!("event poll error: {e}"))? {
                let evt = event::read().map_err(|e| format!("event read error: {e}"))?;
                self.handle_event(evt);
                if self.should_quit {
                    break;
                }
                // After handling keyboard, refresh if 'r' was pressed
                if self.refresh_requested {
                    self.refresh_requested = false;
                    let _ = self.refresh_data_if_needed().await;
                }
            }

            // Auto-refresh every 5 seconds
            if self.last_refresh.elapsed() >= refresh_interval {
                tokio::task::block_in_place(|| {
                    Handle::current().block_on(async {
                        self.refresh_all_data().await;
                    })
                });
                self.last_refresh = Instant::now();
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Event handling
    // -----------------------------------------------------------------------

    fn handle_event(&mut self, evt: Event) {
        match evt {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        self.refresh_requested = true;
                    }
                    // Tab switching: 1-4 direct
                    KeyCode::Char('1') => {
                        self.active_tab = Tab::Dashboard;
                    }
                    KeyCode::Char('2') => {
                        self.active_tab = Tab::Runtimes;
                    }
                    KeyCode::Char('3') => {
                        self.active_tab = Tab::Diagnostics;
                    }
                    KeyCode::Char('4') => {
                        self.active_tab = Tab::History;
                    }
                    // Tab cycling: left/right arrows
                    KeyCode::Left => {
                        let idx = (self.active_tab as usize + 3) % 4;
                        self.active_tab = Tab::from_index(idx);
                    }
                    KeyCode::Right => {
                        let idx = (self.active_tab as usize + 1) % 4;
                        self.active_tab = Tab::from_index(idx);
                    }
                    // Scrolling
                    KeyCode::Char('j') | KeyCode::Down => {
                        let idx = self.active_tab as usize;
                        self.scroll_positions[idx] = self.scroll_positions[idx].saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let idx = self.active_tab as usize;
                        self.scroll_positions[idx] = self.scroll_positions[idx].saturating_sub(1);
                    }
                    KeyCode::Home => {
                        let idx = self.active_tab as usize;
                        self.scroll_positions[idx] = 0;
                    }
                    KeyCode::End => {
                        let idx = self.active_tab as usize;
                        // Will be clamped in rendering
                        self.scroll_positions[idx] = usize::MAX;
                    }
                    _ => {}
                }
            }
            Event::Resize(cols, rows) => {
                self.cols = cols;
                self.rows = rows;
            }
            _ => {}
        }
    }

    /// Called from the event loop when 'r' was pressed — refreshes just the active tab.
    async fn refresh_data_if_needed(&mut self) {
        // Refresh current tab data
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                self.refresh_active_tab().await;
            })
        });
        self.last_refresh = Instant::now();
    }

    /// Refresh data for all tabs.
    async fn refresh_all_data(&mut self) {
        self.fetch_dashboard_data().await;
        self.fetch_runtimes_data().await;
        self.fetch_diagnostics_data().await;
        self.fetch_history_data().await;
    }

    /// Refresh data for the currently active tab only.
    async fn refresh_active_tab(&mut self) {
        match self.active_tab {
            Tab::Dashboard => self.fetch_dashboard_data().await,
            Tab::Runtimes => self.fetch_runtimes_data().await,
            Tab::Diagnostics => self.fetch_diagnostics_data().await,
            Tab::History => self.fetch_history_data().await,
        }
    }

    // -----------------------------------------------------------------------
    // Data fetching
    // -----------------------------------------------------------------------

    async fn fetch_dashboard_data(&mut self) {
        // Get status
        let lifecycle_state = match self.engine.get_status().await {
            Ok(s) => s,
            Err(e) => format!("Error: {e}"),
        };

        // Check config error state
        let toml_path = find_forge_toml(&self.diag_ctx.workspace_root);
        let config_error = if toml_path.is_none() {
            Some("No forge.toml found".to_string())
        } else {
            None
        };

        // Run diagnostics for health score
        let diag_report = {
            let ctx = DiagnosticContext {
                workspace_root: self.diag_ctx.workspace_root.clone(),
                cache_dir: self.diag_ctx.cache_dir.clone(),
                mode: DiagnosticMode::Fast,
                active_profile: self.diag_ctx.active_profile.clone(),
            };
            let mut engine = DiagnosticEngine::new();
            engine
                .register_plugin_checks(self.plugin_health_checks.clone());
            engine.run(&ctx).await
        };
        let health_score = diag_report.health_score;

        // Count runtimes from config and lockfile
        let (runtime_count, ready_runtimes, broken_runtimes) =
            self.count_runtime_status();

        // Last operation
        let last_operation = match self.engine.history(Some(1)).await {
            Ok(mut h) => h.pop(),
            Err(_) => None,
        };

        self.dashboard_data = DashboardData {
            health_score,
            lifecycle_state,
            runtime_count,
            ready_runtimes,
            broken_runtimes,
            last_operation,
            config_error,
        };
    }

    async fn fetch_runtimes_data(&mut self) {
        let toml_path = match find_forge_toml(&self.diag_ctx.workspace_root) {
            Some(p) => p,
            None => {
                self.runtimes_data = RuntimesData {
                    runtimes: Vec::new(),
                    error: Some("No forge.toml found".to_string()),
                };
                return;
            }
        };

        let config = match load_config(&toml_path) {
            Ok(c) => c,
            Err(e) => {
                self.runtimes_data = RuntimesData {
                    runtimes: Vec::new(),
                    error: Some(format!("Failed to parse forge.toml: {e}")),
                };
                return;
            }
        };

        let lock_path = self.diag_ctx.workspace_root.join("forge.lock");
        let lockfile = load_lockfile(&lock_path).ok();

        let mut entries = Vec::new();
        for (name, version_req) in config.runtimes {
            let (version, lock_state) = if let Some(ref lf) = lockfile {
                if let Some(locked) = lf.runtimes.iter().find(|r| r.name == name) {
                    (locked.version.clone(), "Locked".to_string())
                } else {
                    (version_req.clone(), "Unlocked".to_string())
                }
            } else {
                (version_req.clone(), "Missing".to_string())
            };

            // Check cache
            let extracted = self
                .diag_ctx
                .workspace_root
                .join(".forge")
                .join("shims.cache");
            let shim_present = if extracted.exists() {
                if let Ok(content) = std::fs::read_to_string(&extracted) {
                    content.lines().any(|l| {
                        let trimmed = l.trim();
                        !trimmed.is_empty()
                            && !trimmed.starts_with('#')
                            && trimmed.starts_with(&name)
                            && trimmed.contains('=')
                    })
                } else {
                    false
                }
            } else {
                false
            };

            let extract_dir = self
                .diag_ctx
                .cache_dir
                .join(&name)
                .join(&version)
                .join("extracted");
            let cache_path = if extract_dir.exists() {
                Some(extract_dir.to_string_lossy().to_string())
            } else {
                None
            };

            // Determine state
            let state = if cache_path.is_some() && shim_present {
                "Ready".to_string()
            } else if cache_path.is_some() {
                "Synced".to_string()
            } else if lock_state == "Locked" {
                "Locked".to_string()
            } else {
                "Initialized".to_string()
            };

            entries.push(RuntimeEntry {
                name,
                version,
                state,
                lock_state,
                cache_path,
                shim_present,
            });
        }

        self.runtimes_data = RuntimesData {
            runtimes: entries,
            error: None,
        };
    }

    async fn fetch_diagnostics_data(&mut self) {
        let ctx = DiagnosticContext {
            workspace_root: self.diag_ctx.workspace_root.clone(),
            cache_dir: self.diag_ctx.cache_dir.clone(),
            mode: DiagnosticMode::Deep,
            active_profile: self.diag_ctx.active_profile.clone(),
        };

        let mut engine = DiagnosticEngine::new();
        engine
            .register_plugin_checks(self.plugin_health_checks.clone());
        let report = engine.run(&ctx).await;

        self.diagnostics_data = DiagnosticsData {
            score: report.health_score,
            findings: report.findings,
            error: None,
        };
    }

    async fn fetch_history_data(&mut self) {
        match self.engine.history(Some(50)).await {
            Ok(entries) => {
                self.history_data = HistoryData {
                    entries,
                    error: None,
                };
            }
            Err(e) => {
                self.history_data = HistoryData {
                    entries: Vec::new(),
                    error: Some(e),
                };
            }
        }
    }

    /// Count runtimes and their status from config and lockfile.
    fn count_runtime_status(&self) -> (usize, usize, usize) {
        let toml_path = match find_forge_toml(&self.diag_ctx.workspace_root) {
            Some(p) => p,
            None => return (0, 0, 0),
        };
        let config = match load_config(&toml_path) {
            Ok(c) => c,
            Err(_) => return (0, 0, 0),
        };
        let total = config.runtimes.len();
        if total == 0 {
            return (0, 0, 0);
        }

        let lock_path = self.diag_ctx.workspace_root.join("forge.lock");
        let lockfile = load_lockfile(&lock_path).ok();

        let mut ready = 0;
        let mut broken = 0;

        for (name, _) in config.runtimes {
            let version = lockfile
                .as_ref()
                .and_then(|lf| lf.runtimes.iter().find(|r| r.name == name))
                .map(|r| r.version.as_str())
                .unwrap_or("");
            let extract_dir = self
                .diag_ctx
                .cache_dir
                .join(&name)
                .join(version)
                .join("extracted");
            if extract_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&extract_dir) {
                    if entries.next().is_some() {
                        ready += 1;
                        continue;
                    }
                }
                broken += 1;
            } else {
                broken += 1;
            }
        }

        (total, ready, broken)
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.cols = area.width;
        self.rows = area.height;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let main_area = layout[0];
        let tab_bar_area = layout[1];

        // Draw active tab content
        match self.active_tab {
            Tab::Dashboard => self.render_dashboard(frame, main_area),
            Tab::Runtimes => self.render_runtimes(frame, main_area),
            Tab::Diagnostics => self.render_diagnostics(frame, main_area),
            Tab::History => self.render_history(frame, main_area),
        }

        // Draw tab bar
        self.render_tab_bar(frame, tab_bar_area);
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let tabs: Vec<Line> = (0..4)
            .map(|i| {
                let tab = Tab::from_index(i);
                let label = tab.label();
                if tab == self.active_tab {
                    Line::styled(
                        format!(" {} ", label),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Line::styled(
                        format!(" {} ", label),
                        Style::default().fg(Color::DarkGray),
                    )
                }
            })
            .collect();

        let separator = Line::from(Span::raw(" │ "));

        // Interleave tabs with separators
        let mut spans = Vec::new();
        for (i, tab_line) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(separator.clone());
            }
            spans.push(tab_line.clone());
        }

        let bar = Paragraph::new(Text::from(Line::from(
            spans
                .into_iter()
                .flat_map(|l| l.spans.into_iter())
                .collect::<Vec<_>>(),
        )))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_type(BorderType::Plain),
        );

        frame.render_widget(bar, area);
    }

    // -----------------------------------------------------------------------
    // Dashboard View
    // -----------------------------------------------------------------------

    fn render_dashboard(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Min(1),
            ])
            .margin(1)
            .split(area);

        // Health score gauge
        let score = self.dashboard_data.health_score;
        let (gauge_color, score_label) = if score < 50 {
            (Color::Red, "Critical")
        } else if score < 80 {
            (Color::Yellow, "Needs Attention")
        } else {
            (Color::Green, "Healthy")
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(" Health Score ")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(gauge_color))
            .percent(score as u16)
            .label(format!("{score}/100 ({score_label})"));
        frame.render_widget(gauge, layout[0]);

        // Runtime status summary
        let runtime_summary = if let Some(ref err) = self.dashboard_data.config_error {
            format!("  {err}")
        } else {
            format!(
                " {} ({} runtimes, {} ready, {} broken)",
                self.dashboard_data.lifecycle_state,
                self.dashboard_data.runtime_count,
                self.dashboard_data.ready_runtimes,
                self.dashboard_data.broken_runtimes,
            )
        };

        let status = Paragraph::new(runtime_summary)
            .block(Block::default().title(" Runtime Status ").borders(Borders::ALL));
        frame.render_widget(status, layout[1]);

        // Last operation result
        let last_op = match self.dashboard_data.last_operation {
            Some(ref op) => {
                let status_color = match op.status.as_str() {
                    "Success" => Color::Green,
                    "Failure" => Color::Red,
                    "Warning" => Color::Yellow,
                    _ => Color::White,
                };
                Line::from(vec![
                    Span::raw(format!(" ID: {} ", op.id)),
                    Span::raw(format!("| Runtime: {} ", op.runtime)),
                    Span::raw(format!("| Duration: {}ms ", op.duration_ms)),
                    Span::styled(
                        format!("| Status: {} ", op.status),
                        Style::default().fg(status_color),
                    ),
                ])
            }
            None => Line::from(Span::raw(" No operations recorded ")),
        };

        let last_op_widget = Paragraph::new(Text::from(last_op))
            .block(Block::default().title(" Last Operation ").borders(Borders::ALL));
        frame.render_widget(last_op_widget, layout[2]);

        // Keyboard legend
        let legend = Paragraph::new(vec![
            Line::from(" 1-4      Switch tabs"),
            Line::from(" ←/→      Cycle tabs"),
            Line::from(" j/k/↑/↓  Scroll"),
            Line::from(" Home/End Jump to top/bottom"),
            Line::from(" r        Refresh"),
            Line::from(" q/Ctrl+C Quit"),
        ])
        .block(Block::default().title(" Controls ").borders(Borders::ALL));
        frame.render_widget(legend, layout[3]);

        // Empty filler
        let filler = Paragraph::new("")
            .block(Block::default().borders(Borders::ALL).title(" "));
        frame.render_widget(filler, layout[4]);
    }

    // -----------------------------------------------------------------------
    // Runtimes View
    // -----------------------------------------------------------------------

    fn render_runtimes(&self, frame: &mut Frame, area: Rect) {
        let scroll = self.scroll_positions[Tab::Runtimes as usize];

        if let Some(ref err) = self.runtimes_data.error {
            let err_widget = Paragraph::new(Text::styled(
                format!(" ⚠ {err}"),
                Style::default().fg(Color::Red),
            ))
            .block(Block::default().title(" Runtimes ").borders(Borders::ALL));
            frame.render_widget(err_widget, area);
            return;
        }

        if self.runtimes_data.runtimes.is_empty() {
            let empty = Paragraph::new(" No runtimes configured ")
                .block(Block::default().title(" Runtimes ").borders(Borders::ALL));
            frame.render_widget(empty, area);
            return;
        }

        let header_cells = ["Name", "Version", "State", "Cache", "Shim"]
            .iter()
            .map(|h| Cell::from(Line::from(Span::styled(*h, Style::default().add_modifier(Modifier::BOLD)))));
        let header = Row::new(header_cells).height(1).bottom_margin(0);

        let rows: Vec<Row> = self
            .runtimes_data
            .runtimes
            .iter()
            .map(|entry| {
                let state_color = match entry.state.as_str() {
                    "Ready" => Color::Green,
                    "Synced" => Color::Blue,
                    "Broken" => Color::Red,
                    _ => Color::White,
                };
                let cells = vec![
                    Cell::from(entry.name.clone()),
                    Cell::from(entry.version.clone()),
                    Cell::from(
                        Line::from(Span::styled(
                            entry.state.clone(),
                            Style::default().fg(state_color),
                        ))
                        .alignment(ratatui::layout::Alignment::Center),
                    ),
                    Cell::from(
                        if entry.cache_path.is_some() {
                            "✓"
                        } else {
                            "✗"
                        },
                    ),
                    Cell::from(if entry.shim_present { "✓" } else { "✗" }),
                ];
                Row::new(cells).height(1).bottom_margin(0)
            })
            .collect();

        let visible_height = (area.height.max(3) - 3) as usize;
        let offset = scroll.min(
            rows.len().max(1).saturating_sub(visible_height),
        );

        let table = Table::new(
            rows.into_iter()
                .skip(offset)
                .take(visible_height)
                .collect::<Vec<_>>(),
            [
                Constraint::Length(15),
                Constraint::Length(15),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(6),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Runtimes ")
                .borders(Borders::ALL),
        );

        frame.render_widget(table, area);
    }

    // -----------------------------------------------------------------------
    // Diagnostics View
    // -----------------------------------------------------------------------

    fn render_diagnostics(&self, frame: &mut Frame, area: Rect) {
        let scroll = self.scroll_positions[Tab::Diagnostics as usize];

        if let Some(ref err) = self.diagnostics_data.error {
            let err_widget = Paragraph::new(Text::styled(
                format!(" ⚠ {err}"),
                Style::default().fg(Color::Red),
            ))
            .block(
                Block::default()
                    .title(" Diagnostics ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(err_widget, area);
            return;
        }

        // Layout: score header + findings list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .margin(1)
            .split(area);

        // Score header
        let score = self.diagnostics_data.score;
        let (score_color, score_label) = if score < 50 {
            (Color::Red, "Critical")
        } else if score < 80 {
            (Color::Yellow, "Needs Attention")
        } else {
            (Color::Green, "Healthy")
        };
        let score_widget = Paragraph::new(Line::from(vec![
            Span::styled(
                "Health Score: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{score}/100 ({score_label})"),
                Style::default().fg(score_color),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(score_widget, chunks[0]);

        if self.diagnostics_data.findings.is_empty() {
            let empty = Paragraph::new(Text::styled(
                " All healthy — no issues found ",
                Style::default().fg(Color::Green),
            ))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(empty, chunks[1]);
            return;
        }

        // Findings as list items
        let items: Vec<ListItem> = self
            .diagnostics_data
            .findings
            .iter()
            .map(|f| {
                let severity_color = match f.severity {
                    Severity::CRITICAL => Color::Red,
                    Severity::ERROR => Color::Yellow,
                    Severity::WARNING => Color::Blue,
                    Severity::INFO => Color::Green,
                };
                let severity_label = format!("{:?}", f.severity);

                let qf = f
                    .suggested_quick_fix
                    .as_ref()
                    .map(|q| format!(" → Fix: {}", q.description))
                    .unwrap_or_default();

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", f.code),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("[{severity_label}] "),
                            Style::default().fg(severity_color),
                        ),
                        Span::raw(format!("{}", f.message)),
                    ]),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(
                            format!("Confidence: {}%", f.confidence),
                            Style::default().fg(Color::DarkGray),
                        ),
                        if !qf.is_empty() {
                            Span::styled(
                                qf,
                                Style::default().fg(Color::Cyan),
                            )
                        } else {
                            Span::raw("")
                        },
                    ]),
                ])
            })
            .collect();

        let visible_height = (chunks[1].height.max(2) - 2) as usize;
        let offset = scroll.min(
            items.len().max(1).saturating_sub(visible_height),
        );

        let list = List::new(
            items
                .into_iter()
                .skip(offset)
                .take(visible_height)
                .collect::<Vec<_>>(),
        )
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(list, chunks[1]);
    }

    // -----------------------------------------------------------------------
    // History View
    // -----------------------------------------------------------------------

    fn render_history(&self, frame: &mut Frame, area: Rect) {
        let scroll = self.scroll_positions[Tab::History as usize];

        if let Some(ref err) = self.history_data.error {
            let err_widget = Paragraph::new(Text::styled(
                format!(" ⚠ {err}"),
                Style::default().fg(Color::Red),
            ))
            .block(Block::default().title(" History ").borders(Borders::ALL));
            frame.render_widget(err_widget, area);
            return;
        }

        if self.history_data.entries.is_empty() {
            let empty = Paragraph::new(" No operations recorded ")
                .block(Block::default().title(" History ").borders(Borders::ALL));
            frame.render_widget(empty, area);
            return;
        }

        let header_cells = ["Operation ID", "Runtime", "Duration (ms)", "Status"]
            .iter()
            .map(|h| Cell::from(Line::from(Span::styled(*h, Style::default().add_modifier(Modifier::BOLD)))));
        let header = Row::new(header_cells).height(1).bottom_margin(0);

        let rows: Vec<Row> = self
            .history_data
            .entries
            .iter()
            .map(|op| {
                let status_color = match op.status.as_str() {
                    "Success" => Color::Green,
                    "Failure" => Color::Red,
                    "Warning" => Color::Yellow,
                    _ => Color::White,
                };
                let cells = vec![
                    Cell::from(op.id.clone()),
                    Cell::from(op.runtime.clone()),
                    Cell::from(format!("{}", op.duration_ms)),
                    Cell::from(
                        Line::from(Span::styled(
                            op.status.clone(),
                            Style::default().fg(status_color),
                        ))
                        .alignment(ratatui::layout::Alignment::Center),
                    ),
                ];
                Row::new(cells).height(1).bottom_margin(0)
            })
            .collect();

        let visible_height = (area.height.max(3) - 3) as usize;
        let offset = scroll.min(
            rows.len().max(1).saturating_sub(visible_height),
        );

        let table = Table::new(
            rows.into_iter()
                .skip(offset)
                .take(visible_height)
                .collect::<Vec<_>>(),
            [
                Constraint::Length(16),
                Constraint::Length(12),
                Constraint::Length(14),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" History ")
                .borders(Borders::ALL),
        );

        frame.render_widget(table, area);
    }
}

// ---------------------------------------------------------------------------
// Color / state helper functions (exported for testing)
// ---------------------------------------------------------------------------

/// Map a runtime state string to a Ratatui color.
pub fn state_color(state: &str) -> Color {
    match state {
        "Ready" => Color::Green,
        "Synced" => Color::Blue,
        "Broken" => Color::Red,
        "Locked" => Color::Yellow,
        "Initialized" => Color::Cyan,
        _ => Color::White,
    }
}

/// Map a severity level to a Ratatui color.
pub fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::CRITICAL => Color::Red,
        Severity::ERROR => Color::Yellow,
        Severity::WARNING => Color::Blue,
        Severity::INFO => Color::Green,
    }
}

/// Map an operation status string to a Ratatui color.
pub fn operation_status_color(status: &str) -> Color {
    match status {
        "Success" => Color::Green,
        "Failure" => Color::Red,
        "Warning" => Color::Yellow,
        _ => Color::White,
    }
}

/// Classify a health score into (color, label).
pub fn health_score_classify(score: u8) -> (Color, &'static str) {
    if score < 50 {
        (Color::Red, "Critical")
    } else if score < 80 {
        (Color::Yellow, "Needs Attention")
    } else {
        (Color::Green, "Healthy")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use ratatui::style::Color;

    // -----------------------------------------------------------------------
    // Tab switching tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tab_from_index() {
        assert_eq!(Tab::from_index(0), Tab::Dashboard);
        assert_eq!(Tab::from_index(1), Tab::Runtimes);
        assert_eq!(Tab::from_index(2), Tab::Diagnostics);
        assert_eq!(Tab::from_index(3), Tab::History);
        // Out of range wraps
        assert_eq!(Tab::from_index(4), Tab::Dashboard);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(Tab::Dashboard.label(), "1  Dashboard");
        assert_eq!(Tab::Runtimes.label(), "2  Runtimes");
        assert_eq!(Tab::Diagnostics.label(), "3  Diagnostics");
        assert_eq!(Tab::History.label(), "4  History");
    }

    #[test]
    fn test_tab_switching_via_handle_event() {
        // We can test the event handler by creating an App with temp Engine and inspecting state.
        let tmp = std::env::temp_dir().join("forge_tui_test_tab_switch");
        let _ = std::fs::create_dir_all(&tmp);
        let engine = Engine::new(tmp.clone()).unwrap();
        let cache_dir = tmp.join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let diag_ctx = DiagnosticContext {
            workspace_root: tmp.clone(),
            cache_dir,
            mode: DiagnosticMode::Fast,
            active_profile: None,
        };
        let mut app = App::new(engine, diag_ctx, Vec::new());
        assert_eq!(app.active_tab, Tab::Dashboard);

        // Press '2' → Runtimes
        app.handle_event(Event::Key(KeyCode::Char('2').into()));
        assert_eq!(app.active_tab, Tab::Runtimes);

        // Press '4' → History
        app.handle_event(Event::Key(KeyCode::Char('4').into()));
        assert_eq!(app.active_tab, Tab::History);

        // Press '1' → back to Dashboard
        app.handle_event(Event::Key(KeyCode::Char('1').into()));
        assert_eq!(app.active_tab, Tab::Dashboard);

        // Left arrow from Dashboard → History (wraps)
        app.handle_event(Event::Key(KeyCode::Left.into()));
        assert_eq!(app.active_tab, Tab::History);

        // Right arrow from History → Dashboard (wraps)
        app.handle_event(Event::Key(KeyCode::Right.into()));
        assert_eq!(app.active_tab, Tab::Dashboard);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // Quit key tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_quit_key() {
        let tmp = std::env::temp_dir().join("forge_tui_test_quit");
        let _ = std::fs::create_dir_all(&tmp);
        let engine = Engine::new(tmp.clone()).unwrap();
        let cache_dir = tmp.join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let diag_ctx = DiagnosticContext {
            workspace_root: tmp.clone(),
            cache_dir,
            mode: DiagnosticMode::Fast,
            active_profile: None,
        };
        let mut app = App::new(engine, diag_ctx, Vec::new());
        assert!(!app.should_quit);

        app.handle_event(Event::Key(KeyCode::Char('q').into()));
        assert!(app.should_quit);

        // Reset for next test
        app.should_quit = false;
        app.handle_event(Event::Key(
            KeyCode::Char('c').into(),
        ));
        // Ctrl+C is handled by modifier check — this plain 'c' should NOT quit
        assert!(!app.should_quit);

        // Ctrl+C should quit
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.should_quit);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // Scroll tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scroll_state() {
        let tmp = std::env::temp_dir().join("forge_tui_test_scroll");
        let _ = std::fs::create_dir_all(&tmp);
        let engine = Engine::new(tmp.clone()).unwrap();
        let cache_dir = tmp.join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let diag_ctx = DiagnosticContext {
            workspace_root: tmp.clone(),
            cache_dir,
            mode: DiagnosticMode::Fast,
            active_profile: None,
        };
        let mut app = App::new(engine, diag_ctx, Vec::new());

        // Initial scroll positions should be 0
        assert_eq!(app.scroll_positions, [0; 4]);

        // Scroll down on Dashboard
        app.handle_event(Event::Key(KeyCode::Char('j').into()));
        assert_eq!(app.scroll_positions[0], 1);

        // Scroll down more
        app.handle_event(Event::Key(KeyCode::Down.into()));
        assert_eq!(app.scroll_positions[0], 2);

        // Scroll up
        app.handle_event(Event::Key(KeyCode::Char('k').into()));
        assert_eq!(app.scroll_positions[0], 1);

        // Scroll up more (should not go below 0)
        app.handle_event(Event::Key(KeyCode::Up.into()));
        assert_eq!(app.scroll_positions[0], 0);
        app.handle_event(Event::Key(KeyCode::Up.into()));
        assert_eq!(app.scroll_positions[0], 0); // Clamped

        // Home → 0
        app.handle_event(Event::Key(KeyCode::Char('j').into()));
        app.handle_event(Event::Key(KeyCode::Char('j').into()));
        app.handle_event(Event::Key(KeyCode::Home.into()));
        assert_eq!(app.scroll_positions[0], 0);

        // End → usize::MAX (will be clamped in rendering)
        app.handle_event(Event::Key(KeyCode::End.into()));
        assert_eq!(app.scroll_positions[0], usize::MAX);

        // Per-tab scroll isolation
        app.handle_event(Event::Key(KeyCode::Char('2').into()));
        assert_eq!(app.active_tab, Tab::Runtimes);
        assert_eq!(app.scroll_positions[1], 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // Resize event
    // -----------------------------------------------------------------------

    #[test]
    fn test_resize_event() {
        let tmp = std::env::temp_dir().join("forge_tui_test_resize");
        let _ = std::fs::create_dir_all(&tmp);
        let engine = Engine::new(tmp.clone()).unwrap();
        let cache_dir = tmp.join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let diag_ctx = DiagnosticContext {
            workspace_root: tmp.clone(),
            cache_dir,
            mode: DiagnosticMode::Fast,
            active_profile: None,
        };
        let mut app = App::new(engine, diag_ctx, Vec::new());
        app.handle_event(Event::Resize(80, 24));
        assert_eq!(app.cols, 80);
        assert_eq!(app.rows, 24);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -----------------------------------------------------------------------
    // Color helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_state_color() {
        assert_eq!(state_color("Ready"), Color::Green);
        assert_eq!(state_color("Synced"), Color::Blue);
        assert_eq!(state_color("Broken"), Color::Red);
        assert_eq!(state_color("Locked"), Color::Yellow);
        assert_eq!(state_color("Initialized"), Color::Cyan);
        assert_eq!(state_color("Unknown"), Color::White);
    }

    #[test]
    fn test_severity_color() {
        assert_eq!(severity_color(Severity::CRITICAL), Color::Red);
        assert_eq!(severity_color(Severity::ERROR), Color::Yellow);
        assert_eq!(severity_color(Severity::WARNING), Color::Blue);
        assert_eq!(severity_color(Severity::INFO), Color::Green);
    }

    #[test]
    fn test_operation_status_color() {
        assert_eq!(operation_status_color("Success"), Color::Green);
        assert_eq!(operation_status_color("Failure"), Color::Red);
        assert_eq!(operation_status_color("Warning"), Color::Yellow);
        assert_eq!(operation_status_color("Running"), Color::White);
        assert_eq!(operation_status_color(""), Color::White);
    }

    #[test]
    fn test_health_score_classify() {
        let (c1, l1) = health_score_classify(30);
        assert_eq!(c1, Color::Red);
        assert_eq!(l1, "Critical");

        let (c2, l2) = health_score_classify(65);
        assert_eq!(c2, Color::Yellow);
        assert_eq!(l2, "Needs Attention");

        let (c3, l3) = health_score_classify(90);
        assert_eq!(c3, Color::Green);
        assert_eq!(l3, "Healthy");
    }

    // -----------------------------------------------------------------------
    // Compile-check: Event::Key construction helper
    // -----------------------------------------------------------------------

    /// Verify KeyEvent construction works with our usage pattern.
    #[test]
    fn test_key_event_construction() {
        let ev = Event::Key(KeyCode::Char('q').into());
        if let Event::Key(key) = ev {
            assert_eq!(key.code, KeyCode::Char('q'));
        } else {
            panic!("Expected Key event");
        }
    }
}
