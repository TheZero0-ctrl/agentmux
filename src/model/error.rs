use std::error::Error;
use std::fmt;

/// Error returned when a branded model identifier is invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ModelError {
    /// The identifier text was empty or whitespace-only.
    EmptyId,
    /// The identifier text contained a control character.
    ControlCharacter,
    /// The pane identifier did not match tmux `%<ASCII digits>` format.
    InvalidPaneIdFormat,
    /// The display value contained path separators or parent components.
    PathComponent,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("identifier must not be empty"),
            Self::ControlCharacter => {
                formatter.write_str("identifier must not contain control characters")
            }
            Self::InvalidPaneIdFormat => {
                formatter.write_str("pane identifier must match %<ASCII digits>")
            }
            Self::PathComponent => {
                formatter.write_str("display value must be a single path component")
            }
        }
    }
}

impl Error for ModelError {}
