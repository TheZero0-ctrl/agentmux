use super::{
    AgentObservation, AgentState, LocationMetadata, ObservationStatus, PaneId, ProcessEvidence,
    ProcessMetadata,
};

/// A tmux pane with the process evidence visible from discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pane {
    id: PaneId,
    process: ProcessEvidence,
    process_metadata: ProcessMetadata,
    location: LocationMetadata,
}

impl Pane {
    /// Create a pane from a branded id and process evidence.
    #[must_use]
    pub fn new(id: PaneId, process: ProcessEvidence) -> Self {
        let location = LocationMetadata::unknown(id.clone());
        Self {
            id,
            process,
            process_metadata: ProcessMetadata::unknown(),
            location,
        }
    }

    /// Create a pane from parsed privacy-safe location metadata and process evidence.
    #[must_use]
    pub fn with_location(location: LocationMetadata, process: ProcessEvidence) -> Self {
        let id = location.pane_id().clone();
        Self {
            id,
            process,
            process_metadata: ProcessMetadata::unknown(),
            location,
        }
    }

    /// Return this pane with enriched process metadata attached.
    #[must_use]
    pub fn with_process_metadata(self, process_metadata: ProcessMetadata) -> Self {
        Self {
            id: self.id,
            process: self.process,
            process_metadata,
            location: self.location,
        }
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

    /// Return privacy-safe selected process metadata for this pane.
    #[must_use]
    pub const fn process_metadata(&self) -> &ProcessMetadata {
        &self.process_metadata
    }

    /// Create the enriched observation for this pane with unknown new discovery metadata.
    #[must_use]
    pub fn observation(&self, state: AgentState) -> AgentObservation {
        AgentObservation::new(
            self.location.clone(),
            self.process_metadata.clone(),
            ObservationStatus::new(state, self.process.evidence()),
        )
    }
}
