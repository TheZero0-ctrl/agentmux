//! Dashboard runner boundary for agentmux.

use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::{App, DashboardRow};
use crate::daemon::DiscoveryService;
use crate::tmux::SystemTmuxCommand;
use crate::{input, input::KeyInput, projection, tui};

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
    ratatui::run(|terminal| run_with(|| run_dashboard_loop(terminal)))
}

trait DashboardDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>>;
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

fn run_dashboard_loop(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();
    let mut discovery = ProductionDiscovery::new();
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
        if advance_dashboard_action(app, &event) == Some(ActionOutcome::Refresh) {
            refresh_app(app, discovery);
            next_refresh = next_refresh_after(clock.now());
        }

        if !app.is_running() {
            break;
        }
    }

    Ok(())
}

fn next_refresh_after(now: Instant) -> Instant {
    now.checked_add(REFRESH_INTERVAL)
        .map_or(now, |deadline| deadline)
}

fn advance_dashboard_action(app: &mut App, event: &Event) -> Option<ActionOutcome> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            match input::handle_key(map_key_input(key.code, key.modifiers)) {
                Some(crate::app::Action::Quit) => {
                    app.apply(crate::app::Action::Quit);
                    None
                }
                Some(crate::app::Action::Refresh) => Some(ActionOutcome::Refresh),
                None => None,
            }
        }
        _ => None,
    }
}

fn refresh_app(app: &mut App, discovery: &mut impl DashboardDiscovery) {
    match discovery.refresh() {
        Ok(rows) => app.replace_rows(rows),
        Err(_) => app.mark_degraded(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionOutcome {
    Refresh,
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

#[derive(Debug)]
struct ProductionDiscovery {
    service: DiscoveryService<SystemTmuxCommand>,
}

impl ProductionDiscovery {
    fn new() -> Self {
        Self {
            service: DiscoveryService::new(SystemTmuxCommand),
        }
    }
}

impl DashboardDiscovery for ProductionDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        let snapshot = self.service.refresh().map_err(io::Error::other)?;
        Ok(projection::project_snapshot(&snapshot)
            .iter()
            .map(DashboardRow::from_projection)
            .collect())
    }
}

const fn map_key_input(code: KeyCode, modifiers: KeyModifiers) -> KeyInput {
    match code {
        KeyCode::Char('q') if modifiers.is_empty() => KeyInput::Character('q'),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => KeyInput::ControlC,
        KeyCode::Esc => KeyInput::Escape,
        KeyCode::Char(ch) => KeyInput::Character(ch),
        _ => KeyInput::Other,
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
