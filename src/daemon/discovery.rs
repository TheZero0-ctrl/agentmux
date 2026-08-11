//! In-process daemon-owned discovery facade.

use crate::model::{ProcessLiveness, ProcessMetadata};
use crate::process::{
    classify_process_tree, normalize_command_name,
    procfs::{LinuxProcessTreeSource, ProcessRecord},
};
use crate::state::{AgentSnapshot, normalize_snapshot};
use crate::tmux::{TmuxCommand, TmuxError, collect_panes};

/// Injectable process-tree evidence source used to enrich tmux panes.
pub trait ProcessTreeEvidence {
    /// Read privacy-safe process records rooted at a pane PID.
    fn read_tree(&self, root_pid: u32) -> Vec<ProcessRecord>;
}

impl ProcessTreeEvidence for LinuxProcessTreeSource {
    fn read_tree(&self, root_pid: u32) -> Vec<ProcessRecord> {
        self.read_tree(root_pid)
    }
}

/// Discovery facade that owns collection and the previous in-memory snapshot.
#[derive(Debug)]
pub struct DiscoveryService<C, P = LinuxProcessTreeSource> {
    command: C,
    process_source: P,
    previous: AgentSnapshot,
}

impl<C> DiscoveryService<C, LinuxProcessTreeSource>
where
    C: TmuxCommand,
{
    /// Create a discovery service from an injectable tmux command boundary.
    #[must_use]
    pub fn new(command: C) -> Self {
        Self::with_sources(command, LinuxProcessTreeSource::default())
    }
}

impl<C, P> DiscoveryService<C, P>
where
    C: TmuxCommand,
    P: ProcessTreeEvidence,
{
    /// Create a discovery service from injectable tmux and process-tree boundaries.
    #[must_use]
    pub fn with_sources(command: C, process_source: P) -> Self {
        Self {
            command,
            process_source,
            previous: AgentSnapshot::default(),
        }
    }

    /// Return the last successful snapshot retained by the service.
    #[must_use]
    pub const fn snapshot(&self) -> &AgentSnapshot {
        &self.previous
    }

    /// Refresh discovery from tmux and retain the resulting snapshot in memory.
    ///
    /// # Errors
    /// Returns [`TmuxError`] when tmux collection fails.
    pub fn refresh(&mut self) -> Result<AgentSnapshot, TmuxError> {
        let panes = collect_panes(&self.command)?;
        let panes = panes.into_iter().map(|pane| {
            let metadata = match (pane.process().liveness(), pane.process().pid()) {
                (ProcessLiveness::Live, Some(pid)) => {
                    let metadata = classify_process_tree(&self.process_source.read_tree(pid));
                    let foreground =
                        normalize_command_name(pane.process().command().unwrap_or_default());
                    match metadata.candidate().kind() {
                        Some(kind) if foreground != Some(kind.label()) => {
                            ProcessMetadata::unknown()
                        }
                        _ => metadata,
                    }
                }
                (ProcessLiveness::Live, None)
                | (ProcessLiveness::Dead | ProcessLiveness::Unknown, _) => {
                    ProcessMetadata::unknown()
                }
            };
            pane.with_process_metadata(metadata)
        });
        let snapshot = normalize_snapshot(&self.previous, panes);
        self.previous = snapshot.clone();
        Ok(snapshot)
    }
}
