//! In-process daemon-owned discovery facade.

use crate::state::{AgentSnapshot, normalize_snapshot};
use crate::tmux::{TmuxCommand, TmuxError, collect_panes};

/// Discovery facade that owns collection and the previous in-memory snapshot.
#[derive(Debug)]
pub struct DiscoveryService<C> {
    command: C,
    previous: AgentSnapshot,
}

impl<C> DiscoveryService<C>
where
    C: TmuxCommand,
{
    /// Create a discovery service from an injectable tmux command boundary.
    #[must_use]
    pub fn new(command: C) -> Self {
        Self {
            command,
            previous: AgentSnapshot::default(),
        }
    }

    /// Refresh discovery from tmux and retain the resulting snapshot in memory.
    ///
    /// # Errors
    /// Returns [`TmuxError`] when tmux collection fails.
    pub fn refresh(&mut self) -> Result<AgentSnapshot, TmuxError> {
        let panes = collect_panes(&self.command)?;
        let snapshot = normalize_snapshot(&self.previous, panes);
        self.previous = snapshot.clone();
        Ok(snapshot)
    }
}
