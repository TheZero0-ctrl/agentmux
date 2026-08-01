//! Shared presentation projection for discovered agents.

use std::cmp::Ordering;

use crate::model::{
    Agent, AgentState, ClientCandidate, ClientConfidence, EvidenceConfidence, EvidenceFreshness,
    EvidenceSource,
};
use crate::state::AgentSnapshot;

mod filter;

const UNKNOWN: &str = "unknown";

/// Presentation row shared by inspect and future dashboard rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProjectionRow {
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

impl AgentProjectionRow {
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

/// Project a normalized snapshot into deterministically ordered presentation rows.
#[must_use]
pub fn project_snapshot(snapshot: &AgentSnapshot) -> Vec<AgentProjectionRow> {
    let mut rows = snapshot
        .agents()
        .map(project_agent)
        .filter(filter::is_visible_agent_row)
        .collect::<Vec<_>>();
    rows.sort_by(compare_rows);
    rows
}

fn project_agent(agent: &Agent) -> AgentProjectionRow {
    let observation = agent.observation();
    let evidence = observation.evidence();
    let candidate = observation.client_candidate();

    AgentProjectionRow {
        agent_id: observation.agent_id().as_str().to_owned(),
        session_name: UNKNOWN.to_owned(),
        window_index: observation.window_index().get().to_string(),
        window_name: UNKNOWN.to_owned(),
        pane_id: observation.pane_id().as_str().to_owned(),
        pid: observation
            .process_identity()
            .map_or_else(|| UNKNOWN.to_owned(), |identity| identity.pid().to_string()),
        process_name: observation.process_basename().map_or_else(
            || UNKNOWN.to_owned(),
            |basename| basename.as_str().to_owned(),
        ),
        client: candidate.label().to_owned(),
        client_confidence: client_confidence_name(candidate).to_owned(),
        workspace: observation.workspace().as_str().to_owned(),
        state: agent_state_name(observation.state()).to_owned(),
        evidence_source: evidence_source_name(evidence.source()).to_owned(),
        evidence_freshness: evidence_freshness_name(evidence.freshness()).to_owned(),
        evidence_confidence: evidence_confidence_name(evidence.confidence()).to_owned(),
    }
}

fn compare_rows(left: &AgentProjectionRow, right: &AgentProjectionRow) -> Ordering {
    let left_key = PaneSortKey::new(&left.pane_id);
    let right_key = PaneSortKey::new(&right.pane_id);

    left_key
        .significant_digits
        .len()
        .cmp(&right_key.significant_digits.len())
        .then_with(|| {
            left_key
                .significant_digits
                .cmp(right_key.significant_digits)
        })
        .then_with(|| left_key.raw_digits.len().cmp(&right_key.raw_digits.len()))
        .then_with(|| left_key.raw_digits.cmp(right_key.raw_digits))
        .then_with(|| left.agent_id.cmp(&right.agent_id))
}

struct PaneSortKey<'row> {
    raw_digits: &'row str,
    significant_digits: &'row str,
}

impl<'row> PaneSortKey<'row> {
    fn new(pane_id: &'row str) -> Self {
        let raw_digits = pane_id.strip_prefix('%').map_or(pane_id, |digits| digits);
        let significant_digits = significant_digit_suffix(raw_digits);

        Self {
            raw_digits,
            significant_digits,
        }
    }
}

fn significant_digit_suffix(raw_digits: &str) -> &str {
    let trimmed = raw_digits.trim_start_matches('0');

    match trimmed {
        "" => "0",
        digits => digits,
    }
}

const fn agent_state_name(state: AgentState) -> &'static str {
    match state {
        AgentState::Idle => "idle",
        AgentState::Working => "working",
        AgentState::WaitingPermission => "waiting_permission",
        AgentState::WaitingPlanApproval => "waiting_plan_approval",
        AgentState::WaitingQuestion => "waiting_question",
        AgentState::Unknown => "unknown",
        AgentState::Exited => "exited",
    }
}

const fn evidence_source_name(source: EvidenceSource) -> &'static str {
    match source {
        EvidenceSource::Tmux => "tmux",
        EvidenceSource::Process => "process",
        EvidenceSource::MissingPane => "missing_pane",
        EvidenceSource::Hook => "hook",
        EvidenceSource::Marker => "marker",
        EvidenceSource::StructuredLog => "structured_log",
    }
}

const fn evidence_freshness_name(freshness: EvidenceFreshness) -> &'static str {
    match freshness {
        EvidenceFreshness::Fresh => "fresh",
        EvidenceFreshness::Stale => "stale",
    }
}

const fn evidence_confidence_name(confidence: EvidenceConfidence) -> &'static str {
    match confidence {
        EvidenceConfidence::Low => "low",
        EvidenceConfidence::Medium => "medium",
        EvidenceConfidence::High => "high",
    }
}

/// Header shared by inspect, daemon state, and dashboard daemon fallback.
pub const HEADER: &str = "agent_id\tsession_name\twindow_index\twindow_name\tpane_id\tpid\tprocess_name\tclient\tclient_confidence\tworkspace\tstate\tevidence_source\tevidence_freshness\tevidence_confidence";

const fn client_confidence_name(candidate: ClientCandidate) -> &'static str {
    match candidate.confidence() {
        Some(ClientConfidence::Low) => "low",
        None => UNKNOWN,
    }
}
