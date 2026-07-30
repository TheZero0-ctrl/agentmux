//! Application state boundary.

/// User intent applied to the application model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "actions are crate-owned and intentionally exhaustive for deterministic state updates"
)]
pub enum Action {
    /// Stop the application loop.
    Quit,
}

/// Focused application state for the dashboard loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct App {
    running: bool,
}

#[allow(missing_docs, reason = "the public App seam is intentionally tiny")]
impl App {
    pub const fn new() -> Self {
        Self { running: true }
    }

    pub const fn is_running(&self) -> bool {
        self.running
    }

    pub const fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.running = false;
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, App};

    #[test]
    fn initial_state_is_running_when_app_is_created() {
        // Given: a newly created application model.
        let app = App::new();

        // When: the running state is inspected.
        let running = app.is_running();

        // Then: the application is running.
        assert!(running);
    }

    #[test]
    fn running_state_stops_when_quit_is_applied() {
        // Given: a running application model.
        let mut app = App::new();

        // When: the quit action is applied.
        app.apply(Action::Quit);

        // Then: the application is no longer running.
        assert!(!app.is_running());
    }
}
