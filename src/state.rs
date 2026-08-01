//! Deterministic normalization of pane snapshots into agent state.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    Agent, AgentId, AgentState, Evidence, EvidenceConfidence, EvidenceFreshness, EvidenceSource,
    Pane, PaneId, ProcessMetadata,
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
        let agent_id = agent_id_for_lookup(id)?;

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

    pub(crate) fn from_agents(agents: impl IntoIterator<Item = Agent>) -> Self {
        Self {
            agents: agents
                .into_iter()
                .map(|agent| (agent.id().clone(), agent))
                .collect(),
        }
    }
}

fn agent_id_for_lookup(id: &str) -> Option<AgentId> {
    id.strip_prefix("pane:").map_or_else(
        || AgentId::new(id).ok(),
        |pane_id| {
            PaneId::new(pane_id)
                .ok()
                .map(|pane_id| AgentId::from_pane_id(&pane_id))
        },
    )
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
        let state = classify_process(pane.process());

        live_pane_ids.insert(pane_id.clone());
        let agent = Agent::new(pane.observation(state));
        agents.insert(agent.id().clone(), agent);
    }

    for agent in previous.agents.values() {
        if agent.state() == AgentState::Exited
            && agent.evidence().source() == EvidenceSource::MissingPane
        {
            continue;
        }

        if live_pane_ids.contains(agent.pane_id()) {
            continue;
        }

        agents.insert(
            agent.id().clone(),
            Agent::new(
                agent
                    .observation()
                    .with_process_metadata_and_state_evidence(
                        ProcessMetadata::unknown(),
                        AgentState::Exited,
                        missing_pane_evidence(),
                    ),
            ),
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
