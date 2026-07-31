use super::{AgentId, AgentObservation, AgentState, Evidence, PaneId};

/// A normalized agent row owned by the daemon state layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    observation: AgentObservation,
}

impl Agent {
    /// Create a normalized agent row from an enriched observation.
    #[must_use]
    pub(crate) const fn new(observation: AgentObservation) -> Self {
        Self { observation }
    }

    /// Return the enriched observation for shared row projection.
    #[must_use]
    pub const fn observation(&self) -> &AgentObservation {
        &self.observation
    }

    /// Return the agent id.
    #[must_use]
    pub const fn id(&self) -> &AgentId {
        self.observation.agent_id()
    }

    /// Return the pane id that backs this fallback agent.
    #[must_use]
    pub const fn pane_id(&self) -> &PaneId {
        self.observation.pane_id()
    }

    /// Return the normalized state.
    #[must_use]
    pub const fn state(&self) -> AgentState {
        self.observation.state()
    }

    /// Return the evidence used for the normalized state.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.observation.evidence()
    }
}
