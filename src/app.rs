//! Application state boundary.

/// User intent applied to the application model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Action {
    /// Stop the application loop.
    Quit,
    /// Refresh dashboard discovery immediately.
    Refresh,
}

/// Owned dashboard row copied from the shared presentation projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardRow {
    agent_id: String,
    session_name: String,
    window_index: String,
    window_name: String,
    pane_id: String,
    pid: String,
    process_name: String,
    client: String,
    client_confidence: String,
    workspace: String,
    state: String,
    evidence_source: String,
    evidence_freshness: String,
    evidence_confidence: String,
}

/// Focused application state for the dashboard loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct App {
    running: bool,
    rows: Vec<DashboardRow>,
    degraded_message: Option<String>,
}

impl App {
    /// Create an empty running dashboard application state.
    pub const fn new() -> Self {
        Self {
            running: true,
            rows: Vec::new(),
            degraded_message: None,
        }
    }

    /// Return whether the dashboard event loop should continue running.
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Return the currently rendered dashboard rows.
    pub fn rows(&self) -> &[DashboardRow] {
        &self.rows
    }

    /// Return the sanitized degraded refresh message when the last refresh failed.
    pub fn degraded_message(&self) -> Option<&str> {
        self.degraded_message.as_deref()
    }

    /// Replace dashboard rows after a successful refresh.
    pub fn replace_rows(&mut self, rows: Vec<DashboardRow>) {
        self.rows = rows;
        self.degraded_message = None;
    }

    /// Mark refresh as degraded while retaining the last good rows.
    pub fn mark_degraded(&mut self) {
        self.degraded_message = Some("dashboard refresh degraded".to_owned());
    }

    /// Apply a user action to the dashboard state.
    pub const fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.running = false;
            }
            Action::Refresh => {}
        }
    }
}

impl DashboardRow {
    /// Copy a shared projection row into app-owned dashboard state.
    #[must_use]
    pub fn from_projection(row: &crate::projection::AgentProjectionRow) -> Self {
        Self {
            agent_id: row.agent_id().to_owned(),
            session_name: row.session_name().to_owned(),
            window_index: row.window_index().to_owned(),
            window_name: row.window_name().to_owned(),
            pane_id: row.pane_id().to_owned(),
            pid: row.pid().to_owned(),
            process_name: row.process_name().to_owned(),
            client: row.client().to_owned(),
            client_confidence: row.client_confidence().to_owned(),
            workspace: row.workspace().to_owned(),
            state: row.state().to_owned(),
            evidence_source: row.evidence_source().to_owned(),
            evidence_freshness: row.evidence_freshness().to_owned(),
            evidence_confidence: row.evidence_confidence().to_owned(),
        }
    }

    /// Return the pane-derived agent id text.
    #[must_use]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Return the tmux session name.
    #[must_use]
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Return the tmux window index.
    #[must_use]
    pub fn window_index(&self) -> &str {
        &self.window_index
    }

    /// Return the tmux window name.
    #[must_use]
    pub fn window_name(&self) -> &str {
        &self.window_name
    }

    /// Return the backing tmux pane id text.
    #[must_use]
    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    /// Return the selected process id, or unknown.
    #[must_use]
    pub fn pid(&self) -> &str {
        &self.pid
    }

    /// Return the selected process basename, or unknown.
    #[must_use]
    pub fn process_name(&self) -> &str {
        &self.process_name
    }

    /// Return the conservative client candidate label.
    #[must_use]
    pub fn client(&self) -> &str {
        &self.client
    }

    /// Return the conservative client confidence label.
    #[must_use]
    pub fn client_confidence(&self) -> &str {
        &self.client_confidence
    }

    /// Return the privacy-safe workspace label.
    #[must_use]
    pub fn workspace(&self) -> &str {
        &self.workspace
    }

    /// Return the normalized state display text.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// Return the evidence source display text.
    #[must_use]
    pub fn evidence_source(&self) -> &str {
        &self.evidence_source
    }

    /// Return the evidence freshness display text.
    #[must_use]
    pub fn evidence_freshness(&self) -> &str {
        &self.evidence_freshness
    }

    /// Return the evidence confidence display text.
    #[must_use]
    pub fn evidence_confidence(&self) -> &str {
        &self.evidence_confidence
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, App, DashboardRow};

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

    #[test]
    fn given_projected_rows_when_replaced_then_app_owns_rows_and_clears_degraded_message() {
        // Given: a degraded application model and owned projected rows.
        let mut app = App::new();
        app.mark_degraded();
        let rows = vec![DashboardRow::for_test("pane:%1")];

        // When: the successful refresh rows are applied.
        app.replace_rows(rows);

        // Then: the rows are retained and the degraded message is cleared.
        assert_eq!(app.rows(), &[DashboardRow::for_test("pane:%1")]);
        assert_eq!(app.degraded_message(), None);
    }

    #[test]
    fn given_existing_rows_when_refresh_degrades_then_rows_are_retained_with_safe_message() {
        // Given: previously populated dashboard rows.
        let mut app = App::new();
        app.replace_rows(vec![DashboardRow::for_test("pane:%2")]);

        // When: a refresh failure is marked degraded.
        app.mark_degraded();

        // Then: the last good rows remain and no raw error details are stored.
        assert_eq!(app.rows(), &[DashboardRow::for_test("pane:%2")]);
        assert_eq!(app.degraded_message(), Some("dashboard refresh degraded"));
    }

    impl DashboardRow {
        pub(crate) fn for_test(agent_id: &str) -> Self {
            Self {
                agent_id: agent_id.to_owned(),
                session_name: "work".to_owned(),
                window_index: "3".to_owned(),
                window_name: "editor".to_owned(),
                pane_id: "%1".to_owned(),
                pid: "777".to_owned(),
                process_name: "codex".to_owned(),
                client: "codex".to_owned(),
                client_confidence: "low".to_owned(),
                workspace: "project".to_owned(),
                state: "idle".to_owned(),
                evidence_source: "tmux".to_owned(),
                evidence_freshness: "fresh".to_owned(),
                evidence_confidence: "low".to_owned(),
            }
        }

        pub(crate) fn with_location(
            mut self,
            session_name: &str,
            window_index: &str,
            window_name: &str,
        ) -> Self {
            self.session_name = session_name.to_owned();
            self.window_index = window_index.to_owned();
            self.window_name = window_name.to_owned();
            self
        }

        pub(crate) fn with_pane(mut self, pane_id: &str) -> Self {
            self.agent_id = format!("pane:{pane_id}");
            self.pane_id = pane_id.to_owned();
            self
        }

        pub(crate) fn with_process(mut self, pid: &str, process_name: &str) -> Self {
            self.pid = pid.to_owned();
            self.process_name = process_name.to_owned();
            self
        }

        pub(crate) fn with_client(mut self, client: &str, confidence: &str) -> Self {
            self.client = client.to_owned();
            self.client_confidence = confidence.to_owned();
            self
        }

        pub(crate) fn with_workspace(mut self, workspace: &str) -> Self {
            self.workspace = workspace.to_owned();
            self
        }

        pub(crate) fn with_state(mut self, state: &str) -> Self {
            self.state = state.to_owned();
            self
        }

        pub(crate) fn with_evidence(
            mut self,
            source: &str,
            freshness: &str,
            confidence: &str,
        ) -> Self {
            self.evidence_source = source.to_owned();
            self.evidence_freshness = freshness.to_owned();
            self.evidence_confidence = confidence.to_owned();
            self
        }
    }
}
