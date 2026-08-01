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
    assert_eq!(app.selected_index(), Some(0));
}

#[test]
fn given_rows_when_navigation_actions_apply_then_selection_clamps_to_available_rows() {
    // Given: three dashboard rows.
    let mut app = App::new();
    app.replace_rows(vec![
        DashboardRow::for_test("pane:%1"),
        DashboardRow::for_test("pane:%2"),
        DashboardRow::for_test("pane:%3"),
    ]);

    // When: navigation actions move beyond both ends.
    app.apply(Action::SelectNext);
    app.apply(Action::PageNext);
    app.apply(Action::SelectPrevious);
    app.apply(Action::PagePrevious);

    // Then: selection moves and clamps without wrapping.
    assert_eq!(app.selected_index(), Some(0));
    app.apply(Action::SelectLast);
    assert_eq!(app.selected_index(), Some(2));
    app.apply(Action::SelectFirst);
    assert_eq!(app.selected_index(), Some(0));
}

#[test]
fn given_selected_row_when_rows_shrink_then_selection_is_clamped() {
    // Given: a selected row near the end of the list.
    let mut app = App::new();
    app.replace_rows(vec![
        DashboardRow::for_test("pane:%1"),
        DashboardRow::for_test("pane:%2"),
        DashboardRow::for_test("pane:%3"),
    ]);
    app.apply(Action::SelectLast);

    // When: refresh returns fewer rows.
    app.replace_rows(vec![DashboardRow::for_test("pane:%1")]);

    // Then: selection remains valid.
    assert_eq!(app.selected_index(), Some(0));
}

#[test]
fn given_help_action_when_applied_then_help_visibility_toggles() {
    // Given: a dashboard app with hidden help.
    let mut app = App::new();

    // When: help is toggled twice.
    app.apply(Action::ToggleHelp);
    let visible = app.is_help_visible();
    app.apply(Action::ToggleHelp);

    // Then: help opens and closes.
    assert!(visible);
    assert!(!app.is_help_visible());
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

    pub(crate) fn with_evidence(mut self, source: &str, freshness: &str, confidence: &str) -> Self {
        self.evidence_source = source.to_owned();
        self.evidence_freshness = freshness.to_owned();
        self.evidence_confidence = confidence.to_owned();
        self
    }
}
