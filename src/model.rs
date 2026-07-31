//! Typed discovery model for panes, agents, and evidence.

mod agent;
mod candidate;
mod error;
mod evidence;
mod identity;
mod ids;
mod observation;
mod pane;
mod privacy;
mod process;
mod state;

pub use agent::Agent;
pub use candidate::{ClientCandidate, ClientConfidence, ClientKind};
pub use error::ModelError;
pub use evidence::{Evidence, EvidenceConfidence, EvidenceFreshness, EvidenceSource};
pub use identity::ProcessIdentity;
pub use ids::{AgentId, PaneId};
pub use observation::{
    AgentObservation, LocationMetadata, ObservationStatus, ProcessMetadata, SessionWindow,
};
pub use pane::Pane;
pub use privacy::{ProcessBasename, SessionName, WindowIndex, WindowName, WorkspaceLabel};
pub use process::{ProcessEvidence, ProcessLiveness};
pub use state::AgentState;

pub(crate) fn contains_disallowed_display_control(value: &str) -> bool {
    privacy::contains_disallowed_display_control(value)
}
