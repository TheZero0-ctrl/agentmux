use std::fmt;
use std::path::Path;

use super::ModelError;

const UNKNOWN_LABEL: &str = "unknown";

/// Privacy-safe tmux session name.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SessionName(String);

impl SessionName {
    /// Parse a session name from tmux metadata.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        sanitized_component(value).map(Self)
    }

    /// Return the literal unknown session label.
    #[must_use]
    pub fn unknown() -> Self {
        Self(UNKNOWN_LABEL.to_owned())
    }

    /// Return the session label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SessionName")
            .field(&"redacted")
            .finish()
    }
}

/// Zero-based tmux window index.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowIndex(u32);

impl WindowIndex {
    /// Create a typed window index.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the raw tmux window index.
    #[must_use]
    pub const fn get(&self) -> u32 {
        self.0
    }
}

/// Privacy-safe tmux window name.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowName(String);

impl WindowName {
    /// Parse a window name from tmux metadata.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        sanitized_component(value).map(Self)
    }

    /// Return the literal unknown window label.
    #[must_use]
    pub fn unknown() -> Self {
        Self(UNKNOWN_LABEL.to_owned())
    }

    /// Return the window label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for WindowName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WindowName")
            .field(&"redacted")
            .finish()
    }
}

/// Privacy-safe workspace label that stores only a final path component.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceLabel(String);

impl WorkspaceLabel {
    /// Derive a workspace label from an internal path and discard parent components.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when the final component contains a control character.
    pub fn from_path(path: &Path) -> Result<Self, ModelError> {
        let Some(raw_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Ok(Self::unknown());
        };
        let name = final_path_component(raw_name);
        if name.trim().is_empty() {
            return Ok(Self::unknown());
        }
        if contains_disallowed_display_control(name) {
            return Err(ModelError::ControlCharacter);
        }

        Ok(Self(name.to_owned()))
    }

    /// Return the literal unknown workspace label.
    #[must_use]
    pub fn unknown() -> Self {
        Self(UNKNOWN_LABEL.to_owned())
    }

    /// Return the safe workspace label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Debug for WorkspaceLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WorkspaceLabel")
            .field(&"redacted")
            .finish()
    }
}

/// Privacy-safe executable basename, never a path or argv string.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProcessBasename(String);

impl ProcessBasename {
    /// Parse a single executable basename.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    /// Returns [`ModelError::PathComponent`] when `value` contains path separators.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        if contains_path_separator(value) {
            return Err(ModelError::PathComponent);
        }
        sanitized_component(value).map(Self)
    }

    /// Return the process basename.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn sanitized_component(value: &str) -> Result<String, ModelError> {
    if contains_disallowed_display_control(value) {
        return Err(ModelError::ControlCharacter);
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ModelError::EmptyId);
    }

    Ok(trimmed.to_owned())
}

pub(super) fn contains_disallowed_display_control(value: &str) -> bool {
    value.chars().any(is_disallowed_display_control)
}

fn is_disallowed_display_control(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{00ad}'
                | '\u{034f}'
                | '\u{0600}'..='\u{0605}'
                | '\u{061c}'
                | '\u{06dd}'
                | '\u{070f}'
                | '\u{08e2}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{13430}'..='\u{1343f}'
                | '\u{1bca0}'..='\u{1bca3}'
                | '\u{1d173}'..='\u{1d17a}'
        )
}

fn contains_path_separator(value: &str) -> bool {
    value.contains('/') || value.contains('\\')
}

fn final_path_component(value: &str) -> &str {
    value
        .rsplit(['/', '\\'])
        .find(|component| !component.is_empty())
        .unwrap_or(value)
}
