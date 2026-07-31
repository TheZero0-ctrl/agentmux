//! One-shot inspect command output.

use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use crate::daemon::DiscoveryService;
use crate::projection::{AgentProjectionRow, project_snapshot};
use crate::tmux::{TmuxCommand, TmuxError};

/// Error returned by the one-shot inspect command.
#[derive(Debug)]
#[non_exhaustive]
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
    let rows = project_snapshot(&snapshot);

    writeln!(writer, "agentmux inspect")?;
    writeln!(writer, "agents: {}", rows.len())?;

    if rows.is_empty() {
        writeln!(writer, "no agents discovered from tmux panes")?;
        return Ok(());
    }

    writeln!(
        writer,
        "agent_id\tsession_name\twindow_index\twindow_name\tpane_id\tpid\tprocess_name\tclient\tclient_confidence\tworkspace\tstate\tevidence_source\tevidence_freshness\tevidence_confidence"
    )?;

    for row in rows {
        write_agent_row(writer, &row)?;
    }

    Ok(())
}

fn write_agent_row(writer: &mut impl Write, row: &AgentProjectionRow) -> Result<(), InspectError> {
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        row.agent_id(),
        row.session_name(),
        row.window_index(),
        row.window_name(),
        row.pane_id(),
        row.pid(),
        row.process_name(),
        row.client(),
        row.client_confidence(),
        row.workspace(),
        row.state(),
        row.evidence_source(),
        row.evidence_freshness(),
        row.evidence_confidence()
    )?;

    Ok(())
}
