//! Typed discovery model for panes, agents, and evidence.

use std::error::Error;
use std::fmt;

/// Error returned when a branded model identifier is invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ModelError {
    /// The identifier text was empty or whitespace-only.
    EmptyId,
    /// The identifier text contained a control character.
    ControlCharacter,
    /// The pane identifier did not match tmux `%<ASCII digits>` format.
    InvalidPaneIdFormat,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("identifier must not be empty"),
            Self::ControlCharacter => {
                formatter.write_str("identifier must not contain control characters")
            }
            Self::InvalidPaneIdFormat => {
                formatter.write_str("pane identifier must match %<ASCII digits>")
            }
        }
    }
}

impl Error for ModelError {}

/// Branded identifier for a normalized agent row.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AgentId(String);

impl AgentId {
    /// Create an agent identifier from non-empty text.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        if contains_control_character(value) {
            return Err(ModelError::ControlCharacter);
        }
        if value.trim().is_empty() {
            return Err(ModelError::EmptyId);
        }

        Ok(Self(value.to_owned()))
    }

    /// Create the deterministic fallback agent id for a tmux pane.
    pub fn from_pane_id(pane_id: &PaneId) -> Self {
        Self(format!("pane:{}", pane_id.as_str()))
    }

    /// Return the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Branded identifier for a tmux pane.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneId(String);

impl PaneId {
    /// Create a pane identifier from non-empty text.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    /// Returns [`ModelError::InvalidPaneIdFormat`] when `value` is not `%` followed by ASCII digits.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        if contains_control_character(value) {
            return Err(ModelError::ControlCharacter);
        }
        if value.trim().is_empty() {
            return Err(ModelError::EmptyId);
        }
        if !is_tmux_pane_id(value) {
            return Err(ModelError::InvalidPaneIdFormat);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Normalized activity state for an agent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "agent states are spec-owned and intentionally exhaustive"
)]
pub enum AgentState {
    /// The agent appears idle at a shell prompt.
    Idle,
    /// The agent pane has a live non-shell foreground process.
    Working,
    /// The agent is waiting for a permission response.
    WaitingPermission,
    /// The agent is waiting for plan approval.
    WaitingPlanApproval,
    /// The agent is waiting for a question response.
    WaitingQuestion,
    /// The fallback evidence cannot prove a more specific state.
    Unknown,
    /// The pane or process has exited.
    Exited,
}

/// Source that produced the evidence used for a normalized state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "phase 2 evidence sources are deliberately closed and tiny"
)]
pub enum EvidenceSource {
    /// Evidence came from tmux pane metadata.
    Tmux,
    /// Evidence came from generic process metadata.
    Process,
    /// Evidence came from a pane that disappeared from the latest snapshot.
    MissingPane,
}

/// Whether evidence is fresh enough to infer a state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "freshness is a crate-owned binary signal in this phase"
)]
pub enum EvidenceFreshness {
    /// Evidence came from the latest successful snapshot.
    Fresh,
    /// Evidence is retained from an older observation.
    Stale,
}

/// Conservative confidence attached to fallback discovery evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "confidence levels are intentionally limited for fallback evidence"
)]
pub enum EvidenceConfidence {
    /// Evidence is advisory only.
    Low,
    /// Evidence is specific enough to mark absence or exit.
    Medium,
}

/// Evidence metadata attached to a normalized state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Evidence {
    source: EvidenceSource,
    freshness: EvidenceFreshness,
    confidence: EvidenceConfidence,
}

impl Evidence {
    /// Create evidence metadata from a source, freshness, and confidence.
    pub const fn new(
        source: EvidenceSource,
        freshness: EvidenceFreshness,
        confidence: EvidenceConfidence,
    ) -> Self {
        Self {
            source,
            freshness,
            confidence,
        }
    }

    /// Return the evidence source.
    #[must_use]
    pub const fn source(&self) -> EvidenceSource {
        self.source
    }

    /// Return the evidence freshness.
    #[must_use]
    pub const fn freshness(&self) -> EvidenceFreshness {
        self.freshness
    }

    /// Return the evidence confidence.
    #[must_use]
    pub const fn confidence(&self) -> EvidenceConfidence {
        self.confidence
    }
}

/// Whether process evidence says a process is live, dead, or unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::exhaustive_enums,
    reason = "process liveness is crate-owned and fully represented"
)]
pub enum ProcessLiveness {
    /// The process is live.
    Live,
    /// The process is dead.
    Dead,
    /// Liveness could not be proven.
    Unknown,
}

/// Generic process evidence used as fallback state input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessEvidence {
    pid: Option<u32>,
    command: Option<String>,
    liveness: ProcessLiveness,
    evidence: Evidence,
}

impl ProcessEvidence {
    /// Create fresh live process evidence.
    pub fn live(pid: u32, command: &str) -> Self {
        Self::observed(pid, command, ProcessLiveness::Live, EvidenceSource::Process)
    }

    /// Create fresh dead process evidence.
    pub fn dead(pid: u32, command: &str) -> Self {
        Self::observed(pid, command, ProcessLiveness::Dead, EvidenceSource::Process)
    }

    /// Create live process evidence with missing command details.
    pub const fn incomplete_live(pid: u32) -> Self {
        Self {
            pid: Some(pid),
            command: None,
            liveness: ProcessLiveness::Live,
            evidence: Evidence::new(
                EvidenceSource::Process,
                EvidenceFreshness::Fresh,
                EvidenceConfidence::Low,
            ),
        }
    }

    /// Create process evidence from parsed tmux pane metadata.
    pub fn from_tmux(pid: u32, command: &str, liveness: ProcessLiveness) -> Self {
        Self::observed(pid, command, liveness, EvidenceSource::Tmux)
    }

    /// Return a stale copy of this evidence.
    #[must_use]
    pub const fn into_stale(mut self) -> Self {
        self.evidence = Evidence::new(
            self.evidence.source,
            EvidenceFreshness::Stale,
            EvidenceConfidence::Low,
        );
        self
    }

    /// Return the process id when known.
    #[must_use]
    pub const fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Return the command name when known.
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    /// Return the process liveness signal.
    #[must_use]
    pub const fn liveness(&self) -> ProcessLiveness {
        self.liveness
    }

    /// Return the process evidence metadata.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.evidence
    }

    fn observed(
        pid: u32,
        command: &str,
        liveness: ProcessLiveness,
        source: EvidenceSource,
    ) -> Self {
        let command = if command.trim().is_empty() {
            None
        } else {
            Some(command.to_owned())
        };

        Self {
            pid: Some(pid),
            command,
            liveness,
            evidence: Evidence::new(source, EvidenceFreshness::Fresh, confidence_for(liveness)),
        }
    }
}

fn contains_control_character(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn is_tmux_pane_id(value: &str) -> bool {
    value.strip_prefix('%').is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

const fn confidence_for(liveness: ProcessLiveness) -> EvidenceConfidence {
    match liveness {
        ProcessLiveness::Live | ProcessLiveness::Unknown => EvidenceConfidence::Low,
        ProcessLiveness::Dead => EvidenceConfidence::Medium,
    }
}

/// A tmux pane with the process evidence visible from discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pane {
    id: PaneId,
    process: ProcessEvidence,
}

impl Pane {
    /// Create a pane from a branded id and process evidence.
    #[must_use]
    pub const fn new(id: PaneId, process: ProcessEvidence) -> Self {
        Self { id, process }
    }

    /// Return the pane id.
    #[must_use]
    pub const fn id(&self) -> &PaneId {
        &self.id
    }

    /// Return the process evidence for this pane.
    #[must_use]
    pub const fn process(&self) -> &ProcessEvidence {
        &self.process
    }
}

/// A normalized agent row owned by the daemon state layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    pub(crate) id: AgentId,
    pub(crate) pane_id: PaneId,
    pub(crate) state: AgentState,
    pub(crate) evidence: Evidence,
}

impl Agent {
    /// Return the agent id.
    #[must_use]
    pub const fn id(&self) -> &AgentId {
        &self.id
    }

    /// Return the pane id that backs this fallback agent.
    #[must_use]
    pub const fn pane_id(&self) -> &PaneId {
        &self.pane_id
    }

    /// Return the normalized state.
    #[must_use]
    pub const fn state(&self) -> AgentState {
        self.state
    }

    /// Return the evidence used for the normalized state.
    #[must_use]
    pub const fn evidence(&self) -> Evidence {
        self.evidence
    }
}
