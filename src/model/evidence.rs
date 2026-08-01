/// Source that produced the evidence used for a normalized state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvidenceSource {
    /// Evidence came from tmux pane metadata.
    Tmux,
    /// Evidence came from generic process metadata.
    Process,
    /// Evidence came from a pane that disappeared from the latest snapshot.
    MissingPane,
    /// Evidence came from an agent hook event.
    Hook,
    /// Evidence came from an explicit marker event.
    Marker,
    /// Evidence came from a structured log event.
    StructuredLog,
}

/// Whether evidence is fresh enough to infer a state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvidenceFreshness {
    /// Evidence came from the latest successful snapshot.
    Fresh,
    /// Evidence is retained from an older observation.
    Stale,
}

/// Conservative confidence attached to fallback discovery evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvidenceConfidence {
    /// Evidence is advisory only.
    Low,
    /// Evidence is specific enough to mark absence or exit.
    Medium,
    /// Evidence is authoritative for the current daemon revision.
    High,
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
