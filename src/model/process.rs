use std::fmt;

use super::{Evidence, EvidenceConfidence, EvidenceFreshness, EvidenceSource};

/// Whether process evidence says a process is live, dead, or unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProcessLiveness {
    /// The process is live.
    Live,
    /// The process is dead.
    Dead,
    /// Liveness could not be proven.
    Unknown,
}

/// Generic process evidence used as fallback state input.
#[derive(Clone, Eq, PartialEq)]
pub struct ProcessEvidence {
    pid: Option<u32>,
    command: Option<String>,
    liveness: ProcessLiveness,
    evidence: Evidence,
}

impl fmt::Debug for ProcessEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessEvidence")
            .field("pid", &self.pid)
            .field(
                "command",
                &self.command.as_ref().map(|_command| "<redacted>"),
            )
            .field("liveness", &self.liveness)
            .field("evidence", &self.evidence)
            .finish()
    }
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
            self.evidence.source(),
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
        let command = command.trim();
        let command = if command.is_empty() {
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

const fn confidence_for(liveness: ProcessLiveness) -> EvidenceConfidence {
    match liveness {
        ProcessLiveness::Live | ProcessLiveness::Unknown => EvidenceConfidence::Low,
        ProcessLiveness::Dead => EvidenceConfidence::Medium,
    }
}
