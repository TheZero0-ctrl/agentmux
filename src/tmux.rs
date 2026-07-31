//! tmux list-panes command boundary and strict parser facade.

mod command;
mod parser;

use std::error::Error;
use std::fmt;

use crate::model::Pane;

pub use command::{SystemTmuxCommand, TmuxCommand};
pub use parser::parse_list_panes;

const FIELD_SEPARATOR: char = '\u{1f}';
const LIST_PANES_FORMAT: &str = "#{session_name}\u{1f}#{window_index}\u{1f}#{window_name}\u{1f}#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}";

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
    },
    /// A row did not contain exactly the required fields.
    MalformedFieldCount {
        /// Zero-based row index in `tmux list-panes` output.
        row_index: usize,
        /// Number of fields found in the row.
        fields: usize,
    },
    /// A specific row field could not be parsed into its typed model value.
    MalformedField {
        /// Zero-based row index in `tmux list-panes` output.
        row_index: usize,
        /// Static tmux field name.
        field_name: &'static str,
        /// Privacy-safe parse failure reason.
        reason: FieldErrorReason,
    },
}

/// Privacy-safe reason for a malformed tmux field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldErrorReason {
    /// Field contained a non-separator control character.
    ControlCharacter,
    /// Field was empty where tmux metadata requires a value.
    Empty,
    /// Field was not a valid unsigned integer.
    InvalidUnsignedInteger,
    /// Field was not one of tmux's `0` or `1` booleans.
    InvalidBoolean,
    /// Field was not a tmux `%<digits>` pane id.
    InvalidPaneId,
    /// Field contained path syntax where a display component was required.
    PathComponent,
}

impl TmuxError {
    /// Create a command failure while discarding tmux stderr.
    #[must_use]
    pub const fn command_failed(status: Option<i32>, _stderr: &str) -> Self {
        Self::CommandFailed { status }
    }
}

impl fmt::Display for FieldErrorReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlCharacter => formatter.write_str("control character"),
            Self::Empty => formatter.write_str("empty"),
            Self::InvalidUnsignedInteger => formatter.write_str("invalid unsigned integer"),
            Self::InvalidBoolean => formatter.write_str("invalid boolean"),
            Self::InvalidPaneId => formatter.write_str("invalid pane id"),
            Self::PathComponent => formatter.write_str("path component"),
        }
    }
}

impl fmt::Display for TmuxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandIo { source } => write!(formatter, "failed to execute tmux: {source}"),
            Self::CommandFailed { status } => {
                formatter.write_str("tmux list-panes failed with status ")?;
                match status {
                    Some(code) => write!(formatter, "{code}")?,
                    None => formatter.write_str("unknown")?,
                }
                formatter.write_str(": stderr discarded")
            }
            Self::MalformedFieldCount { row_index, fields } => {
                write!(formatter, "tmux row {row_index} has {fields} fields")
            }
            Self::MalformedField {
                row_index,
                field_name,
                reason,
            } => write!(
                formatter,
                "tmux row {row_index} field {field_name} is invalid: {reason}"
            ),
        }
    }
}

impl Error for TmuxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CommandIo { source } => Some(source),
            Self::CommandFailed { .. }
            | Self::MalformedFieldCount { .. }
            | Self::MalformedField { .. } => None,
        }
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
