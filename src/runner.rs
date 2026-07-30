//! Dashboard runner boundary for agentmux.

use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::{app::App, input, input::KeyInput, tui};

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

/// Advance the dashboard by one input event.
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

    while app.is_running() {
        terminal.draw(|frame| tui::render(frame, &app))?;

        let event = event::read()?;
        if !advance_dashboard_event(&mut app, &event) {
            break;
        }
    }

    Ok(())
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
mod tests {
    use std::io;

    use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::app::App;

    use super::{advance_dashboard_event, run_with};

    #[test]
    fn given_injected_error_when_runner_is_invoked_then_error_is_propagated() {
        // Given: a runner seam that injects a failing step.
        // When: the runner executes the injected step.
        // Then: the injected error should surface to the caller.
        let result = run_with(|| Err(io::Error::other("boom")));
        assert!(result.is_err());
    }

    #[test]
    fn given_quit_key_event_when_advanced_then_app_stops_running_and_loop_exits() {
        // Given: a running dashboard and a quit key event.
        let mut app = App::new();
        let event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        // When: the runner advances one event step.
        let continue_running = advance_dashboard_event(&mut app, &event);

        // Then: the app stops running and the loop exits.
        assert!(!app.is_running());
        assert!(!continue_running);
    }

    #[test]
    fn given_resize_event_when_advanced_then_app_keeps_running() {
        // Given: a running dashboard and a non-key event.
        let mut app = App::new();
        let event = Event::Resize(80, 24);

        // When: the runner advances one event step.
        let continue_running = advance_dashboard_event(&mut app, &event);

        // Then: the app keeps running and the loop continues.
        assert!(app.is_running());
        assert!(continue_running);
    }
}
