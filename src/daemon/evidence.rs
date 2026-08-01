//! Structured authoritative evidence accepted by the local daemon.

use std::fmt;

use crate::model::{AgentId, AgentState, EvidenceSource, PaneId};

/// Maximum accepted evidence request body size.
pub const MAX_EVIDENCE_BODY_BYTES: usize = 4096;

/// Ordered source for authoritative daemon evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthoritativeSource {
    /// Agent hook evidence.
    Hook,
    /// Explicit marker evidence.
    Marker,
    /// Structured log evidence.
    StructuredLog,
}

impl AuthoritativeSource {
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::StructuredLog => 1,
            Self::Marker => 2,
            Self::Hook => 3,
        }
    }

    pub(crate) const fn evidence_source(self) -> EvidenceSource {
        match self {
            Self::Hook => EvidenceSource::Hook,
            Self::Marker => EvidenceSource::Marker,
            Self::StructuredLog => EvidenceSource::StructuredLog,
        }
    }
}

/// Waiting state asserted by structured daemon evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthoritativeState {
    /// Waiting for permission approval.
    WaitingPermission,
    /// Waiting for plan approval.
    WaitingPlanApproval,
    /// Waiting for an answer to a question.
    WaitingQuestion,
    /// Clear the current waiting override.
    Clear,
}

impl AuthoritativeState {
    pub(crate) const fn agent_state(self) -> Option<AgentState> {
        match self {
            Self::WaitingPermission => Some(AgentState::WaitingPermission),
            Self::WaitingPlanApproval => Some(AgentState::WaitingPlanApproval),
            Self::WaitingQuestion => Some(AgentState::WaitingQuestion),
            Self::Clear => None,
        }
    }
}

/// Parsed authoritative evidence event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoritativeEvent {
    source: AuthoritativeSource,
    agent_id: AgentId,
    state: AuthoritativeState,
    sequence: u64,
}

impl AuthoritativeEvent {
    /// Create a parsed authoritative event.
    #[must_use]
    pub const fn new(
        source: AuthoritativeSource,
        agent_id: AgentId,
        state: AuthoritativeState,
        sequence: u64,
    ) -> Self {
        Self {
            source,
            agent_id,
            state,
            sequence,
        }
    }

    /// Return the event source.
    #[must_use]
    pub const fn source(&self) -> AuthoritativeSource {
        self.source
    }

    /// Return the target agent id.
    #[must_use]
    pub const fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Return the asserted state.
    #[must_use]
    pub const fn state(&self) -> AuthoritativeState {
        self.state
    }

    /// Return the source-local sequence number.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// Sanitized structured evidence parse error.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvidenceParseError {
    /// Request body exceeded the accepted size.
    BodyTooLarge,
    /// A line did not start with the supported version token.
    InvalidVersion,
    /// A required field was absent.
    MissingField(&'static str),
    /// A field name or value was unsupported.
    InvalidField(&'static str),
    /// The event contained an unknown field.
    UnknownField,
}

impl fmt::Display for EvidenceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BodyTooLarge => write!(formatter, "evidence body too large"),
            Self::InvalidVersion => write!(formatter, "invalid evidence version"),
            Self::MissingField(field) => write!(formatter, "missing evidence field: {field}"),
            Self::InvalidField(field) => write!(formatter, "invalid evidence field: {field}"),
            Self::UnknownField => write!(formatter, "unknown evidence field"),
        }
    }
}

impl std::error::Error for EvidenceParseError {}

/// Parse newline-delimited structured evidence lines.
pub fn parse_evidence_body(body: &str) -> Result<Vec<AuthoritativeEvent>, EvidenceParseError> {
    if body.len() > MAX_EVIDENCE_BODY_BYTES {
        return Err(EvidenceParseError::BodyTooLarge);
    }
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_evidence_line)
        .collect()
}

/// Parse one structured evidence line.
pub fn parse_evidence_line(line: &str) -> Result<AuthoritativeEvent, EvidenceParseError> {
    let mut parts = line.split_ascii_whitespace();
    match parts.next() {
        Some("agentmux.v1") => {}
        Some(_) | None => return Err(EvidenceParseError::InvalidVersion),
    }

    let mut source = None;
    let mut agent_id = None;
    let mut state = None;
    let mut sequence = None;

    for part in parts {
        let (key, value) = part
            .split_once('=')
            .ok_or(EvidenceParseError::UnknownField)?;
        match key {
            "source" => source = Some(parse_source(value)?),
            "agent_id" => agent_id = Some(parse_agent_id(value)?),
            "state" => state = Some(parse_state(value)?),
            "sequence" => sequence = Some(parse_sequence(value)?),
            _other => return Err(EvidenceParseError::UnknownField),
        }
    }

    Ok(AuthoritativeEvent::new(
        source.ok_or(EvidenceParseError::MissingField("source"))?,
        agent_id.ok_or(EvidenceParseError::MissingField("agent_id"))?,
        state.ok_or(EvidenceParseError::MissingField("state"))?,
        sequence.ok_or(EvidenceParseError::MissingField("sequence"))?,
    ))
}

fn parse_source(value: &str) -> Result<AuthoritativeSource, EvidenceParseError> {
    match value {
        "hook" => Ok(AuthoritativeSource::Hook),
        "marker" => Ok(AuthoritativeSource::Marker),
        "structured_log" => Ok(AuthoritativeSource::StructuredLog),
        _other => Err(EvidenceParseError::InvalidField("source")),
    }
}

fn parse_agent_id(value: &str) -> Result<AgentId, EvidenceParseError> {
    let pane_id = value
        .strip_prefix("pane:")
        .ok_or(EvidenceParseError::InvalidField("agent_id"))?;
    PaneId::new(pane_id)
        .map(|pane_id| AgentId::from_pane_id(&pane_id))
        .map_err(|_error| EvidenceParseError::InvalidField("agent_id"))
}

fn parse_state(value: &str) -> Result<AuthoritativeState, EvidenceParseError> {
    match value {
        "waiting_permission" => Ok(AuthoritativeState::WaitingPermission),
        "waiting_plan_approval" => Ok(AuthoritativeState::WaitingPlanApproval),
        "waiting_question" => Ok(AuthoritativeState::WaitingQuestion),
        "clear" => Ok(AuthoritativeState::Clear),
        _other => Err(EvidenceParseError::InvalidField("state")),
    }
}

fn parse_sequence(value: &str) -> Result<u64, EvidenceParseError> {
    value
        .parse::<u64>()
        .map_err(|_error| EvidenceParseError::InvalidField("sequence"))
}
