//! One-shot inspect command output.

use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use crate::daemon::DiscoveryService;
use crate::model::{Agent, AgentState, EvidenceConfidence, EvidenceFreshness, EvidenceSource};
use crate::tmux::{TmuxCommand, TmuxError};

/// Error returned by the one-shot inspect command.
#[derive(Debug)]
#[allow(
    clippy::exhaustive_enums,
    reason = "inspect errors are crate-owned and intentionally closed"
)]
pub enum InspectError {
    /// Discovery failed while reading tmux state.
    Tmux {
        /// Source discovery error.
        source: TmuxError,
    },
    /// Writing inspect output failed.
    Io {
        /// Source writer error.
        source: io::Error,
    },
}

impl fmt::Display for InspectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tmux { source } => write!(formatter, "inspect discovery failed: {source}"),
            Self::Io { source } => write!(formatter, "inspect output failed: {source}"),
        }
    }
}

impl Error for InspectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Tmux { source } => Some(source),
            Self::Io { source } => Some(source),
        }
    }
}

impl InspectError {
    pub(crate) fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Tmux { source: _source } => false,
            Self::Io { source } => source.kind() == io::ErrorKind::BrokenPipe,
        }
    }
}

impl From<TmuxError> for InspectError {
    fn from(source: TmuxError) -> Self {
        Self::Tmux { source }
    }
}

impl From<io::Error> for InspectError {
    fn from(source: io::Error) -> Self {
        Self::Io { source }
    }
}

/// Run a single tmux-derived discovery pass and write normalized inspect output.
///
/// # Errors
/// Returns [`InspectError`] when discovery or output writing fails.
pub fn run_inspect(command: impl TmuxCommand, writer: &mut impl Write) -> Result<(), InspectError> {
    let mut discovery = DiscoveryService::new(command);
    let snapshot = discovery.refresh()?;

    writeln!(writer, "agentmux inspect")?;
    writeln!(writer, "agents: {}", snapshot.len())?;

    if snapshot.is_empty() {
        writeln!(writer, "no agents discovered from tmux panes")?;
        return Ok(());
    }

    writeln!(
        writer,
        "agent_id\tpane_id\tstate\tevidence_source\tevidence_freshness\tevidence_confidence"
    )?;

    let mut agents = snapshot.agents().collect::<Vec<_>>();
    agents.sort_by(|left, right| compare_agent_rows(left, right));

    for agent in agents {
        write_agent_row(writer, agent)?;
    }

    Ok(())
}

fn write_agent_row(writer: &mut impl Write, agent: &Agent) -> Result<(), InspectError> {
    let evidence = agent.evidence();

    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}",
        agent.id().as_str(),
        agent.pane_id().as_str(),
        agent_state_name(agent.state()),
        evidence_source_name(evidence.source()),
        evidence_freshness_name(evidence.freshness()),
        evidence_confidence_name(evidence.confidence())
    )?;

    Ok(())
}

fn compare_agent_rows(left: &Agent, right: &Agent) -> std::cmp::Ordering {
    let left_key = pane_sort_key(left);
    let right_key = pane_sort_key(right);

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
        .then_with(|| left.id().as_str().cmp(right.id().as_str()))
}

fn pane_sort_key(agent: &Agent) -> PaneSortKey<'_> {
    let raw_digits = agent
        .pane_id()
        .as_str()
        .strip_prefix('%')
        .map_or_else(|| agent.pane_id().as_str(), |digits| digits);
    let significant_digits = significant_digit_suffix(raw_digits);

    PaneSortKey {
        raw_digits,
        significant_digits,
    }
}

fn significant_digit_suffix(raw_digits: &str) -> &str {
    let trimmed = raw_digits.trim_start_matches('0');

    match trimmed {
        "" => "0",
        digits => digits,
    }
}

struct PaneSortKey<'agent> {
    raw_digits: &'agent str,
    significant_digits: &'agent str,
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
    }
}
