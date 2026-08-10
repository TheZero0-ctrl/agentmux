//! Application state boundary.

use std::collections::BTreeSet;

use ansi_to_tui::IntoText;
use ratatui::text::Text;

/// User intent applied to the application model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Action {
    /// Stop the application loop.
    Quit,
    /// Refresh dashboard discovery immediately.
    Refresh,
    /// Select the next dashboard row.
    SelectNext,
    /// Select the previous dashboard row.
    SelectPrevious,
    /// Select the first dashboard row.
    SelectFirst,
    /// Select the last dashboard row.
    SelectLast,
    /// Move selection down by a page-sized step.
    PageNext,
    /// Move selection up by a page-sized step.
    PagePrevious,
    /// Toggle the dashboard help overlay.
    ToggleHelp,
    /// Toggle the agent sidebar.
    ToggleSidebar,
    /// Toggle keyboard input forwarding for the selected agent.
    ToggleInputMode,
    /// Switch the tmux client to the selected agent's real pane.
    OpenSelectedPane,
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
    content: String,
    rendered_content: Option<Text<'static>>,
}

/// Focused application state for the dashboard loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct App {
    running: RunningState,
    rows: Vec<DashboardRow>,
    degraded_message: Option<String>,
    selected_index: usize,
    help: Visibility,
    sidebar: Visibility,
    input: InputMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunningState {
    Running,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Visibility {
    Visible,
    Hidden,
}

impl Visibility {
    const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }

    const fn toggled(self) -> Self {
        match self {
            Self::Visible => Self::Hidden,
            Self::Hidden => Self::Visible,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    Forwarding,
    Browsing,
}

impl InputMode {
    const fn is_forwarding(self) -> bool {
        matches!(self, Self::Forwarding)
    }
}

impl App {
    /// Create an empty running dashboard application state.
    pub const fn new() -> Self {
        Self {
            running: RunningState::Running,
            rows: Vec::new(),
            degraded_message: None,
            selected_index: 0,
            help: Visibility::Hidden,
            sidebar: Visibility::Visible,
            input: InputMode::Browsing,
        }
    }

    /// Return whether the dashboard event loop should continue running.
    pub const fn is_running(&self) -> bool {
        matches!(self.running, RunningState::Running)
    }

    /// Return the currently rendered dashboard rows.
    pub fn rows(&self) -> &[DashboardRow] {
        &self.rows
    }

    /// Return the selected dashboard row index when rows are available.
    pub const fn selected_index(&self) -> Option<usize> {
        if self.rows.is_empty() {
            None
        } else {
            Some(self.selected_index)
        }
    }

    /// Return the selected dashboard row when rows are available.
    pub fn selected_row(&self) -> Option<&DashboardRow> {
        self.selected_index()
            .and_then(|selected_index| self.rows.get(selected_index))
    }

    /// Return whether the help overlay is visible.
    pub const fn is_help_visible(&self) -> bool {
        self.help.is_visible()
    }

    /// Return whether the agent sidebar is visible.
    pub const fn is_sidebar_visible(&self) -> bool {
        self.sidebar.is_visible()
    }

    /// Return the selected visible agent ID for focused preview hydration.
    #[must_use]
    pub fn selected_preview_agent_ids(&self) -> BTreeSet<String> {
        self.selected_row().map_or_else(BTreeSet::new, |row| {
            std::iter::once(row.agent_id().to_owned()).collect()
        })
    }

    /// Return whether keyboard input is being forwarded to the selected agent.
    pub const fn is_input_mode(&self) -> bool {
        self.input.is_forwarding()
    }

    /// Return the sanitized degraded refresh message when the last refresh failed.
    pub fn degraded_message(&self) -> Option<&str> {
        self.degraded_message.as_deref()
    }

    /// Replace dashboard rows after a successful refresh.
    pub fn replace_rows(&mut self, rows: Vec<DashboardRow>) {
        let mut rows = rows;
        // Keep keyboard traversal in the same order users see in the grouped
        // sidebar: project/workspace first, then stable pane identity.
        rows.sort_by(|left, right| {
            left.workspace
                .cmp(&right.workspace)
                .then_with(|| left.pane_id.cmp(&right.pane_id))
                .then_with(|| left.agent_id.cmp(&right.agent_id))
        });
        self.rows = rows;
        self.degraded_message = None;
        self.clamp_selection();
    }

    /// Mark refresh as degraded while retaining the last good rows.
    pub fn mark_degraded(&mut self) {
        self.degraded_message = Some("dashboard refresh degraded".to_owned());
    }

    /// Apply a user action to the dashboard state.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.running = RunningState::Stopped;
            }
            Action::Refresh | Action::OpenSelectedPane => {}
            Action::SelectNext => self.move_selection(1),
            Action::SelectPrevious => self.move_selection(-1),
            Action::SelectFirst => self.selected_index = 0,
            Action::SelectLast => self.select_last(),
            Action::PageNext => self.move_selection(5),
            Action::PagePrevious => self.move_selection(-5),
            Action::ToggleHelp => self.help = self.help.toggled(),
            Action::ToggleSidebar => self.sidebar = self.sidebar.toggled(),
            Action::ToggleInputMode => {
                self.input = if self.is_input_mode() || self.selected_row().is_none() {
                    InputMode::Browsing
                } else {
                    InputMode::Forwarding
                };
            }
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.rows.is_empty() {
            self.selected_index = 0;
            return;
        }
        self.selected_index = self
            .selected_index
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
    }

    const fn select_last(&mut self) {
        self.selected_index = self.rows.len().saturating_sub(1);
    }

    fn clamp_selection(&mut self) {
        if self.rows.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = self.selected_index.min(self.rows.len().saturating_sub(1));
        }
    }
}

impl DashboardRow {
    /// Copy daemon TSV fields into app-owned dashboard state.
    #[must_use]
    pub fn from_tsv_fields(fields: &[&str]) -> Option<Self> {
        let [
            agent_id,
            session_name,
            window_index,
            window_name,
            pane_id,
            pid,
            process_name,
            client,
            client_confidence,
            workspace,
            state,
            evidence_source,
            evidence_freshness,
            evidence_confidence,
        ] = fields
        else {
            return None;
        };

        Some(Self {
            agent_id: (*agent_id).to_owned(),
            session_name: (*session_name).to_owned(),
            window_index: (*window_index).to_owned(),
            window_name: (*window_name).to_owned(),
            pane_id: (*pane_id).to_owned(),
            pid: (*pid).to_owned(),
            process_name: (*process_name).to_owned(),
            client: (*client).to_owned(),
            client_confidence: (*client_confidence).to_owned(),
            workspace: (*workspace).to_owned(),
            state: (*state).to_owned(),
            evidence_source: (*evidence_source).to_owned(),
            evidence_freshness: (*evidence_freshness).to_owned(),
            evidence_confidence: (*evidence_confidence).to_owned(),
            content: String::new(),
            rendered_content: None,
        })
    }

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
            content: String::new(),
            rendered_content: None,
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

    /// Return the captured local pane text, or an empty string when unavailable.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Return cached styled pane content for rendering.
    pub const fn rendered_content(&self) -> Option<&Text<'static>> {
        self.rendered_content.as_ref()
    }

    /// Attach captured local pane text without changing the row identity.
    pub fn set_content(&mut self, content: String) {
        self.rendered_content = content.as_bytes().to_vec().into_text().ok();
        self.content = content;
    }

    /// Update the display state after terminal-content classification.
    pub fn set_state(&mut self, state: &str) {
        state.clone_into(&mut self.state);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
