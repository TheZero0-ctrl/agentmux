const UNKNOWN_LABEL: &str = "unknown";

/// Known client basenames supported by local discovery hints.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ClientKind {
    /// `OpenCode` CLI basename `opencode`.
    OpenCode,
    /// Codex CLI basename `codex`.
    Codex,
    /// Claude Code basename `claude`.
    Claude,
    /// Gemini CLI basename `gemini`.
    Gemini,
}

impl ClientKind {
    /// Return the privacy-safe client label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }
}

/// Confidence assigned to a candidate client hint.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ClientConfidence {
    /// The basename is only an advisory hint, not verified identity.
    Low,
}

/// Conservative local client candidate classification.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ClientCandidate {
    /// A known basename matched with explicit confidence.
    Known {
        /// Candidate client kind.
        kind: ClientKind,
        /// Confidence for this hint.
        confidence: ClientConfidence,
    },
    /// Candidate evidence is absent, incomplete, generic, or conflicting.
    Unknown,
}

impl ClientCandidate {
    /// Create a known client candidate hint.
    #[must_use]
    pub const fn known(kind: ClientKind, confidence: ClientConfidence) -> Self {
        Self::Known { kind, confidence }
    }

    /// Create an unknown candidate classification.
    #[must_use]
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    /// Return the candidate kind when known.
    #[must_use]
    pub const fn kind(&self) -> Option<ClientKind> {
        match self {
            Self::Known {
                kind,
                confidence: _,
            } => Some(*kind),
            Self::Unknown => None,
        }
    }

    /// Return candidate confidence when known.
    #[must_use]
    pub const fn confidence(&self) -> Option<ClientConfidence> {
        match self {
            Self::Known {
                kind: _,
                confidence,
            } => Some(*confidence),
            Self::Unknown => None,
        }
    }

    /// Return the privacy-safe candidate label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Known {
                kind,
                confidence: _,
            } => kind.label(),
            Self::Unknown => UNKNOWN_LABEL,
        }
    }
}
