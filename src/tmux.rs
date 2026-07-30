//! tmux list-panes command boundary and strict parser.

use std::error::Error;
use std::fmt;
use std::process::Command;

use crate::model::{Pane, PaneId, ProcessEvidence, ProcessLiveness};

const FIELD_SEPARATOR: char = '\u{1f}';
const LIST_PANES_FORMAT: &str =
    "#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_command}";

/// Output returned by the injectable tmux command boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TmuxOutput {
    stdout: String,
}

impl TmuxOutput {
    /// Create tmux output from stdout text.
    #[must_use]
    pub const fn new(stdout: String) -> Self {
        Self { stdout }
    }

    /// Return stdout as text.
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }
}

/// Error returned by tmux execution or strict parsing.
#[derive(Debug)]
#[non_exhaustive]
pub enum TmuxError {
    /// tmux could not be executed.
    CommandIo {
        /// Source I/O error from `std::process::Command`.
        source: std::io::Error,
    },
    /// tmux exited unsuccessfully.
    CommandFailed {
        /// Process exit status code when available.
        status: Option<i32>,
        /// Standard error emitted by tmux.
        stderr: String,
    },
    /// A row did not contain all required fields.
    MalformedFieldCount {
        /// Original malformed row.
        row: String,
        /// Number of fields found in the row.
        fields: usize,
    },
    /// A pane pid field was not a valid process id.
    MalformedPid {
        /// Original malformed row.
        row: String,
        /// Invalid pid field value.
        value: String,
    },
    /// A pane dead field was not `0` or `1`.
    MalformedBoolean {
        /// Original malformed row.
        row: String,
        /// Invalid boolean field value.
        value: String,
    },
    /// A parsed pane id was empty.
    InvalidPaneId {
        /// Original malformed row.
        row: String,
    },
}

impl fmt::Display for TmuxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandIo { source } => write!(formatter, "failed to execute tmux: {source}"),
            Self::CommandFailed { status, stderr } => {
                formatter.write_str("tmux list-panes failed with status ")?;
                match status {
                    Some(code) => write!(formatter, "{code}")?,
                    None => formatter.write_str("unknown")?,
                }
                write!(formatter, ": {stderr}")
            }
            Self::MalformedFieldCount { row, fields } => {
                write!(formatter, "tmux row has {fields} fields: {row}")
            }
            Self::MalformedPid { row, value } => {
                write!(formatter, "tmux row has invalid pid {value}: {row}")
            }
            Self::MalformedBoolean { row, value } => {
                write!(formatter, "tmux row has invalid pane_dead {value}: {row}")
            }
            Self::InvalidPaneId { row } => write!(formatter, "tmux row has invalid pane id: {row}"),
        }
    }
}

impl Error for TmuxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CommandIo { source } => Some(source),
            Self::CommandFailed { .. }
            | Self::MalformedFieldCount { .. }
            | Self::MalformedPid { .. }
            | Self::MalformedBoolean { .. }
            | Self::InvalidPaneId { .. } => None,
        }
    }
}

/// Injectable boundary for collecting tmux pane rows.
pub trait TmuxCommand {
    /// Run `tmux list-panes` with the supplied format string.
    ///
    /// # Errors
    /// Returns [`TmuxError`] when command execution fails.
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError>;
}

/// Production tmux command runner.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct SystemTmuxCommand;

impl TmuxCommand for SystemTmuxCommand {
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError> {
        let output = Command::new("tmux")
            .args(["list-panes", "-a", "-F", format])
            .output()
            .map_err(|source| TmuxError::CommandIo { source })?;

        if output.status.success() {
            return Ok(TmuxOutput::new(
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ));
        }

        Err(TmuxError::CommandFailed {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

/// Collect panes through an injectable tmux command boundary.
///
/// # Errors
/// Returns [`TmuxError`] when tmux execution or parsing fails.
pub fn collect_panes(command: &impl TmuxCommand) -> Result<Vec<Pane>, TmuxError> {
    let output = command.list_panes(LIST_PANES_FORMAT)?;
    parse_list_panes(output.stdout())
}

/// Parse strict delimiter-separated `tmux list-panes` output.
///
/// # Errors
/// Returns [`TmuxError`] when any row is malformed.
pub fn parse_list_panes(output: &str) -> Result<Vec<Pane>, TmuxError> {
    let mut panes = Vec::new();

    for row in output.lines() {
        panes.push(parse_row(row)?);
    }

    Ok(panes)
}

fn parse_row(row: &str) -> Result<Pane, TmuxError> {
    let mut fields = row.splitn(4, FIELD_SEPARATOR);
    let pane_id = next_field(&mut fields, row, RequiredField::Id)?;
    let pid = next_field(&mut fields, row, RequiredField::Pid)?;
    let dead = next_field(&mut fields, row, RequiredField::Dead)?;
    let command = next_field(&mut fields, row, RequiredField::CurrentCommand)?;

    let pane_id = PaneId::new(pane_id).map_err(|_error| TmuxError::InvalidPaneId {
        row: row.to_owned(),
    })?;
    let pid = pid
        .parse::<u32>()
        .map_err(|_error| TmuxError::MalformedPid {
            row: row.to_owned(),
            value: pid.to_owned(),
        })?;
    let liveness = parse_liveness(dead, row)?;

    Ok(Pane::new(
        pane_id,
        ProcessEvidence::from_tmux(pid, command, liveness),
    ))
}

fn next_field<'row>(
    fields: &mut impl Iterator<Item = &'row str>,
    row: &str,
    required_field: RequiredField,
) -> Result<&'row str, TmuxError> {
    fields.next().ok_or_else(|| TmuxError::MalformedFieldCount {
        row: row.to_owned(),
        fields: required_field.previous_field_count(),
    })
}

#[derive(Clone, Copy, Debug)]
enum RequiredField {
    Id,
    Pid,
    Dead,
    CurrentCommand,
}

impl RequiredField {
    const fn previous_field_count(self) -> usize {
        match self {
            Self::Id => 0,
            Self::Pid => 1,
            Self::Dead => 2,
            Self::CurrentCommand => 3,
        }
    }
}

fn parse_liveness(value: &str, row: &str) -> Result<ProcessLiveness, TmuxError> {
    match value {
        "0" => Ok(ProcessLiveness::Live),
        "1" => Ok(ProcessLiveness::Dead),
        _ => Err(TmuxError::MalformedBoolean {
            row: row.to_owned(),
            value: value.to_owned(),
        }),
    }
}
