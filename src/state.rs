//! Deterministic normalization of pane snapshots into agent state.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    Agent, AgentId, AgentState, Evidence, EvidenceConfidence, EvidenceFreshness, EvidenceSource,
    Pane,
};
use crate::process::classify_process;

/// Deterministic snapshot of daemon-owned fallback agent state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentSnapshot {
    agents: BTreeMap<AgentId, Agent>,
}

impl AgentSnapshot {
    /// Return the number of normalized agents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Return whether the snapshot contains no normalized agents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Iterate over normalized agents in deterministic id order.
    pub fn agents(&self) -> impl ExactSizeIterator<Item = &Agent> {
        self.agents.values()
    }

    /// Return a normalized agent by id text.
    #[must_use]
    pub fn agent(&self, id: &str) -> Option<&Agent> {
        let agent_id = match AgentId::new(id) {
            Ok(agent_id) => agent_id,
            Err(
                crate::model::ModelError::EmptyId
                | crate::model::ModelError::ControlCharacter
                | crate::model::ModelError::InvalidPaneIdFormat,
            ) => return None,
        };

        self.agents.get(&agent_id)
    }

    /// Return a normalized agent state by id text.
    #[must_use]
    pub fn agent_state(&self, id: &str) -> Option<AgentState> {
        self.agent(id).map(Agent::state)
    }

    /// Return agent ids in deterministic order.
    #[must_use]
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().cloned().collect()
    }
}

/// Normalize the latest successful pane snapshot into deterministic agent state.
#[must_use]
pub fn normalize_snapshot(
    previous: &AgentSnapshot,
    panes: impl IntoIterator<Item = Pane>,
) -> AgentSnapshot {
    let mut agents = BTreeMap::new();
    let mut live_pane_ids = BTreeSet::new();

    for pane in panes {
        let pane_id = pane.id().clone();
        let agent_id = AgentId::from_pane_id(&pane_id);
        let state = classify_process(pane.process());
        let evidence = pane.process().evidence();

        live_pane_ids.insert(pane_id.clone());
        agents.insert(
            agent_id.clone(),
            Agent {
                id: agent_id,
                pane_id,
                state,
                evidence,
            },
        );
    }

    for agent in previous.agents.values() {
        if agent.state() == AgentState::Exited {
            continue;
        }

        if live_pane_ids.contains(agent.pane_id()) {
            continue;
        }

        agents.insert(
            agent.id().clone(),
            Agent {
                id: agent.id().clone(),
                pane_id: agent.pane_id().clone(),
                state: AgentState::Exited,
                evidence: missing_pane_evidence(),
            },
        );
    }

    AgentSnapshot { agents }
}

const fn missing_pane_evidence() -> Evidence {
    Evidence::new(
        EvidenceSource::MissingPane,
        EvidenceFreshness::Fresh,
        EvidenceConfidence::Medium,
    )
}
