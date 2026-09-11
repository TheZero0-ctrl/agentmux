//! Dashboard runner boundary for agentmux.

use std::collections::BTreeSet;
use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::{App, DashboardRow};
use crate::{input, input::KeyInput, tui};

mod discovery;

use discovery::ProductionDiscovery;

const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const MAX_POLL_TIMEOUT: Duration = Duration::from_millis(100);

/// Run an injected dashboard step and return its result unchanged.
fn run_with<E, F>(perform_step: F) -> Result<(), E>
where
    F: FnOnce() -> Result<(), E>,
{
    perform_step()
}

/// Run the dashboard through Ratatui's terminal boundary.
pub fn run_dashboard() -> io::Result<()> {
    run_dashboard_with_options(DashboardOptions::default())
}

/// Run the dashboard through Ratatui's terminal boundary with explicit options.
pub fn run_dashboard_with_options(options: DashboardOptions) -> io::Result<()> {
    ratatui::run(|terminal| run_with(|| run_dashboard_loop(terminal, options)))
}

/// Dashboard runtime options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DashboardOptions {
    daemon: DashboardDaemonMode,
}

impl DashboardOptions {
    /// Create dashboard options from the CLI daemon autostart switch.
    #[must_use]
    pub const fn from_auto_start_daemon(auto_start_daemon: bool) -> Self {
        let daemon = if auto_start_daemon {
            DashboardDaemonMode::AutoStart
        } else {
            DashboardDaemonMode::ExistingOnly
        };
        Self { daemon }
    }

    pub(super) const fn daemon(self) -> DashboardDaemonMode {
        self.daemon
    }
}

impl Default for DashboardOptions {
    fn default() -> Self {
        Self::from_auto_start_daemon(true)
    }
}

/// Dashboard daemon lifecycle mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DashboardDaemonMode {
    /// Spawn a dashboard-owned daemon only when the default endpoint is unreachable.
    AutoStart,
    /// Use an already-running daemon if it is available, but never spawn one.
    ExistingOnly,
}

trait DashboardDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>>;

    fn refresh_visible(
        &mut self,
        _visible_agent_ids: &BTreeSet<String>,
    ) -> io::Result<Vec<DashboardRow>> {
        self.refresh()
    }

    fn forward_key(&mut self, _pane_id: &str, _event: &Event) -> io::Result<()> {
        Ok(())
    }

    fn switch_client_to_pane(&mut self, _pane_id: &str) -> io::Result<()> {
        Ok(())
    }
}

trait Clock {
    fn now(&self) -> Instant;

    #[cfg(test)]
    fn advance(&self, _duration: Duration) {}
}

trait DashboardEvents {
    fn poll(&mut self, timeout: Duration, clock: &impl Clock) -> io::Result<bool>;

    fn read(&mut self) -> io::Result<Event>;
}

trait DashboardRenderer {
    fn draw(&mut self, app: &App) -> io::Result<()>;
}

/// Advance the dashboard by one input event.
#[cfg(test)]
fn advance_dashboard_event(app: &mut App, event: &Event) -> bool {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if let Some(action) = input::handle_key(map_key_input(key.code, key.modifiers)) {
                app.apply(action);
            }

            app.is_running()
        }
        _ => true,
    }
}

fn run_dashboard_loop(
    terminal: &mut ratatui::DefaultTerminal,
    options: DashboardOptions,
) -> io::Result<()> {
    let mut app = App::new();
    let mut discovery = ProductionDiscovery::new(options);
    let mut events = CrosstermEvents;
    let clock = SystemClock;
    let mut renderer = TerminalRenderer { terminal };

    run_dashboard_loop_with(&mut app, &mut discovery, &mut events, &clock, &mut renderer)
}

fn run_dashboard_loop_with<D, E, C, R>(
    app: &mut App,
    discovery: &mut D,
    events: &mut E,
    clock: &C,
    renderer: &mut R,
) -> io::Result<()>
where
    D: DashboardDiscovery,
    E: DashboardEvents,
    C: Clock,
    R: DashboardRenderer,
{
    refresh_app(app, discovery);
    let mut next_refresh = next_refresh_after(clock.now());

    while app.is_running() {
        if clock.now() >= next_refresh {
            refresh_app(app, discovery);
            next_refresh = next_refresh_after(clock.now());
        }

        renderer.draw(app)?;

        let timeout = MAX_POLL_TIMEOUT.min(next_refresh.saturating_duration_since(clock.now()));
        if !events.poll(timeout, clock)? {
            continue;
        }

        let event = events.read()?;
        if app.is_input_mode() {
            if is_escape_key(&event) {
                app.apply(crate::app::Action::ToggleInputMode);
            } else if let Some(row) = app.selected_row() {
                if discovery.forward_key(row.pane_id(), &event).is_ok() {
                    refresh_app(app, discovery);
                    next_refresh = next_refresh_after(clock.now());
                } else {
                    app.mark_degraded();
                }
            }
            continue;
        }
        match advance_dashboard_action(app, &event) {
            Some(ActionOutcome::Refresh) => {
                refresh_app(app, discovery);
                next_refresh = next_refresh_after(clock.now());
            }
            Some(ActionOutcome::SwitchClientToPane(pane_id)) => {
                if discovery.switch_client_to_pane(&pane_id).is_ok() {
                    // Leave agentmux running in its original pane. The tmux
                    // client now shows the selected real pane, and the user
                    // can return to this dashboard with normal tmux navigation.
                } else {
                    app.mark_degraded();
                }
            }
            None => {}
        }

        if !app.is_running() {
            break;
        }
    }

    Ok(())
}

fn is_escape_key(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(key)
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Esc
    )
}

fn next_refresh_after(now: Instant) -> Instant {
    now.checked_add(REFRESH_INTERVAL).unwrap_or(now)
}

fn advance_dashboard_action(app: &mut App, event: &Event) -> Option<ActionOutcome> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            match input::handle_key(map_key_input(key.code, key.modifiers)) {
                Some(crate::app::Action::Refresh) => Some(ActionOutcome::Refresh),
                Some(crate::app::Action::OpenSelectedPane) => app
                    .selected_row()
                    .map(|row| ActionOutcome::SwitchClientToPane(row.pane_id().to_owned())),
                Some(
                    action @ (crate::app::Action::SelectNext
                    | crate::app::Action::SelectPrevious
                    | crate::app::Action::SelectFirst
                    | crate::app::Action::SelectLast
                    | crate::app::Action::PageNext
                    | crate::app::Action::PagePrevious),
                ) => {
                    app.apply(action);
                    Some(ActionOutcome::Refresh)
                }
                Some(action) => {
                    app.apply(action);
                    None
                }
                None => None,
            }
        }
        _ => None,
    }
}

fn refresh_app(app: &mut App, discovery: &mut impl DashboardDiscovery) {
    let visible_agent_ids = app.selected_preview_agent_ids();
    let result = if app.rows().is_empty() {
        discovery.refresh()
    } else {
        discovery.refresh_visible(&visible_agent_ids)
    };
    match result {
        Ok(rows) => app.replace_rows(rows),
        Err(_) => app.mark_degraded(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActionOutcome {
    Refresh,
    SwitchClientToPane(String),
}

#[derive(Clone, Copy, Debug, Default)]
struct CrosstermEvents;

impl DashboardEvents for CrosstermEvents {
    fn poll(&mut self, timeout: Duration, _clock: &impl Clock) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        event::read()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct TerminalRenderer<'terminal> {
    terminal: &'terminal mut ratatui::DefaultTerminal,
}

impl DashboardRenderer for TerminalRenderer<'_> {
    fn draw(&mut self, app: &App) -> io::Result<()> {
        self.terminal.draw(|frame| tui::render(frame, app))?;
        Ok(())
    }
}

const fn map_key_input(code: KeyCode, modifiers: KeyModifiers) -> KeyInput {
    match code {
        KeyCode::Char('q') if modifiers.is_empty() => KeyInput::Character('q'),
        KeyCode::Tab => KeyInput::Tab,
        KeyCode::Enter => KeyInput::Enter,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => KeyInput::ControlC,
        KeyCode::Esc => KeyInput::Escape,
        KeyCode::Up => KeyInput::Up,
        KeyCode::Down => KeyInput::Down,
        KeyCode::Home => KeyInput::Home,
        KeyCode::End => KeyInput::End,
        KeyCode::PageUp => KeyInput::PageUp,
        KeyCode::PageDown => KeyInput::PageDown,
        KeyCode::Char(ch) => KeyInput::Character(ch),
        _ => KeyInput::Other,
    }
}

#[cfg(test)]
mod autostart_tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
