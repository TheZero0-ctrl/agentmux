use super::{
    AgentId, ClientCandidate, Evidence, PaneId, ProcessBasename, ProcessIdentity, SessionName,
    WindowIndex, WindowName, WorkspaceLabel,
};

/// Typed session and window metadata for a pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionWindow {
    session_name: SessionName,
    window_index: WindowIndex,
    window_name: WindowName,
}

impl SessionWindow {
    /// Create typed session/window metadata.
    #[must_use]
    pub const fn new(
        session_name: SessionName,
        window_index: WindowIndex,
        window_name: WindowName,
    ) -> Self {
        Self {
            session_name,
            window_index,
            window_name,
        }
    }

    /// Return unknown session/window metadata.
    #[must_use]
    pub fn unknown() -> Self {
        Self::new(
            SessionName::unknown(),
            WindowIndex::new(0),
            WindowName::unknown(),
        )
    }

    /// Return the session name.
    #[must_use]
    pub const fn session_name(&self) -> &SessionName {
        &self.session_name
    }

    /// Return the window index.
    #[must_use]
    pub const fn window_index(&self) -> WindowIndex {
        self.window_index
    }

    /// Return the window name.
    #[must_use]
    pub const fn window_name(&self) -> &WindowName {
        &self.window_name
    }
}

/// Privacy-safe location metadata for an observed pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocationMetadata {
    pane_id: PaneId,
    session_window: SessionWindow,
    workspace: WorkspaceLabel,
}

impl LocationMetadata {
    /// Create location metadata from typed pane/session/window/workspace values.
    #[must_use]
    pub const fn new(
        pane_id: PaneId,
        session_window: SessionWindow,
        workspace: WorkspaceLabel,
    ) -> Self {
        Self {
            pane_id,
            session_window,
            workspace,
        }
    }

    /// Return location metadata for an unknown pane context.
    #[must_use]
    pub fn unknown(pane_id: PaneId) -> Self {
        Self::new(pane_id, SessionWindow::unknown(), WorkspaceLabel::unknown())
    }

    /// Return the pane id.
    #[must_use]
    pub const fn pane_id(&self) -> &PaneId {
        &self.pane_id
    }

    /// Return the session/window metadata.
    #[must_use]
    pub const fn session_window(&self) -> &SessionWindow {
        &self.session_window
    }

    /// Return the workspace label.
    #[must_use]
    pub const fn workspace(&self) -> &WorkspaceLabel {
        &self.workspace
    }
}

/// Privacy-safe selected process metadata for an observation row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessMetadata {
    identity: Option<ProcessIdentity>,
    basename: Option<ProcessBasename>,
    candidate: ClientCandidate,
}

impl ProcessMetadata {
    /// Create process metadata from parsed safe components.
    #[must_use]
    pub const fn new(
        identity: Option<ProcessIdentity>,
        basename: Option<ProcessBasename>,
        candidate: ClientCandidate,
    ) -> Self {
        Self {
            identity,
            basename,
            candidate,
        }
    }

    /// Return unknown process metadata.
    #[must_use]
    pub const fn unknown() -> Self {
        Self::new(None, None, ClientCandidate::Unknown)
    }

    /// Return the selected process identity when known.
    #[must_use]
    pub const fn identity(&self) -> Option<&ProcessIdentity> {
        self.identity.as_ref()
    }

    /// Return the selected process basename when known.
    #[must_use]
    pub const fn basename(&self) -> Option<&ProcessBasename> {
        self.basename.as_ref()
    }

    /// Return candidate classification.
    #[must_use]
    pub const fn candidate(&self) -> ClientCandidate {
        self.candidate
    }
}

/// State and evidence attached to an observation row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationStatus {
    state: super::AgentState,
    evidence: Evidence,
}

impl ObservationStatus {
    /// Create status metadata from normalized state and evidence.
    #[must_use]
    pub const fn new(state: super::AgentState, evidence: Evidence) -> Self {
        Self { state, evidence }
    }

    /// Return the normalized state.
    #[must_use]
    pub const fn state(&self) -> super::AgentState {
        self.state
    }

    /// Return the evidence for the normalized state.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.evidence
    }
}

/// Shared enriched agent observation consumed by future inspect and dashboard rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentObservation {
    agent_id: AgentId,
    location: LocationMetadata,
    process: ProcessMetadata,
    status: ObservationStatus,
}

impl AgentObservation {
    /// Create an enriched observation from privacy-safe model parts.
    #[must_use]
    pub fn new(
        location: LocationMetadata,
        process: ProcessMetadata,
        status: ObservationStatus,
    ) -> Self {
        let agent_id = AgentId::from_pane_id(location.pane_id());
        Self {
            agent_id,
            location,
            process,
            status,
        }
    }

    /// Return a copy of this observation with updated state evidence.
    #[must_use]
    pub fn with_state_evidence(&self, state: super::AgentState, evidence: Evidence) -> Self {
        Self {
            agent_id: self.agent_id.clone(),
            location: self.location.clone(),
            process: self.process.clone(),
            status: ObservationStatus::new(state, evidence),
        }
    }

    /// Return a copy of this observation with updated process metadata and state evidence.
    #[must_use]
    pub fn with_process_metadata_and_state_evidence(
        &self,
        process: ProcessMetadata,
        state: super::AgentState,
        evidence: Evidence,
    ) -> Self {
        Self {
            agent_id: self.agent_id.clone(),
            location: self.location.clone(),
            process,
            status: ObservationStatus::new(state, evidence),
        }
    }

    /// Return the pane-derived agent id.
    #[must_use]
    pub const fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Return the backing pane id.
    #[must_use]
    pub const fn pane_id(&self) -> &PaneId {
        self.location.pane_id()
    }

    /// Return the tmux session name.
    #[must_use]
    pub const fn session_name(&self) -> &SessionName {
        self.location.session_window().session_name()
    }

    /// Return the tmux window index.
    #[must_use]
    pub const fn window_index(&self) -> WindowIndex {
        self.location.session_window().window_index()
    }

    /// Return the tmux window name.
    #[must_use]
    pub const fn window_name(&self) -> &WindowName {
        self.location.session_window().window_name()
    }

    /// Return the sanitized workspace label.
    #[must_use]
    pub const fn workspace(&self) -> &WorkspaceLabel {
        self.location.workspace()
    }

    /// Return the selected process identity when known.
    #[must_use]
    pub const fn process_identity(&self) -> Option<&ProcessIdentity> {
        self.process.identity()
    }

    /// Return the selected executable basename when known.
    #[must_use]
    pub const fn process_basename(&self) -> Option<&ProcessBasename> {
        self.process.basename()
    }

    /// Return client candidate classification.
    #[must_use]
    pub const fn client_candidate(&self) -> ClientCandidate {
        self.process.candidate()
    }

    /// Return the normalized state.
    #[must_use]
    pub const fn state(&self) -> super::AgentState {
        self.status.state()
    }

    /// Return the evidence for the normalized state.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.status.evidence()
    }
}
