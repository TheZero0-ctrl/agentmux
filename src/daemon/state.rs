//! Authoritative daemon state reducer.

use std::collections::BTreeMap;

use crate::model::{Agent, AgentId, AgentState, Evidence, EvidenceConfidence, EvidenceFreshness};
use crate::state::AgentSnapshot;

use super::evidence::{AuthoritativeEvent, AuthoritativeSource, AuthoritativeState};

/// In-memory reducer overlaying authoritative waiting evidence onto fallback discovery.
#[derive(Clone, Debug, Default)]
pub struct DaemonState {
    fallback: AgentSnapshot,
    overrides: BTreeMap<AgentId, EvidenceSlot>,
    revision: u64,
}

impl DaemonState {
    /// Return the current monotonic revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Replace the fallback discovery snapshot and prune evidence for absent agents.
    pub fn reconcile_fallback(&mut self, snapshot: AgentSnapshot) {
        self.overrides
            .retain(|agent_id, _slot| snapshot.agent(agent_id.as_str()).is_some());
        self.fallback = snapshot;
        self.bump_revision();
    }

    /// Ingest one authoritative event.
    pub fn ingest(&mut self, event: &AuthoritativeEvent) {
        let agent_id = event.agent_id().clone();
        let replacement = EvidenceSlot::from_event(event);
        let changed = self
            .overrides
            .get(&agent_id)
            .is_none_or(|current| replacement.beats(current));

        if changed {
            self.overrides.insert(agent_id, replacement);
            self.bump_revision();
        }
    }

    /// Return the effective snapshot after authoritative overlays are applied.
    #[must_use]
    pub fn effective_snapshot(&self) -> AgentSnapshot {
        AgentSnapshot::from_agents(
            self.fallback
                .agents()
                .map(|agent| self.overlay_agent(agent)),
        )
    }

    fn overlay_agent(&self, agent: &Agent) -> Agent {
        if agent.state() == AgentState::Exited {
            return agent.clone();
        }

        self.overrides
            .get(agent.id())
            .and_then(EvidenceSlot::active_state)
            .map_or_else(
                || agent.clone(),
                |(state, source)| {
                    Agent::new(agent.observation().with_state_evidence(
                        state,
                        Evidence::new(
                            source.evidence_source(),
                            EvidenceFreshness::Fresh,
                            EvidenceConfidence::High,
                        ),
                    ))
                },
            )
    }

    const fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EvidenceSlot {
    source: AuthoritativeSource,
    state: AuthoritativeState,
    sequence: u64,
}

impl EvidenceSlot {
    const fn from_event(event: &AuthoritativeEvent) -> Self {
        Self {
            source: event.source(),
            state: event.state(),
            sequence: event.sequence(),
        }
    }

    const fn active_state(&self) -> Option<(AgentState, AuthoritativeSource)> {
        match self.state.agent_state() {
            Some(state) => Some((state, self.source)),
            None => None,
        }
    }

    const fn beats(self, current: &Self) -> bool {
        if self.source.rank() > current.source.rank() {
            return true;
        }
        if self.source.rank() < current.source.rank() {
            return false;
        }
        self.sequence > current.sequence
    }
}
