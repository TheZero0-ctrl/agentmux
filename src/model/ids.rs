use super::{ModelError, contains_disallowed_display_control};

/// Branded identifier for a normalized agent row.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AgentId(String);

impl AgentId {
    /// Create an agent identifier from non-empty text.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        if contains_disallowed_display_control(value) {
            return Err(ModelError::ControlCharacter);
        }
        if value.trim().is_empty() {
            return Err(ModelError::EmptyId);
        }
        if value.starts_with("pane:") {
            return Err(ModelError::InvalidPaneIdFormat);
        }

        Ok(Self(value.to_owned()))
    }

    /// Create the deterministic fallback agent id for a tmux pane.
    pub fn from_pane_id(pane_id: &PaneId) -> Self {
        Self(format!("pane:{}", pane_id.as_str()))
    }

    /// Return the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Branded identifier for a tmux pane.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneId(String);

impl PaneId {
    /// Create a pane identifier from non-empty text.
    ///
    /// # Errors
    /// Returns [`ModelError::ControlCharacter`] when `value` contains a control character.
    /// Returns [`ModelError::EmptyId`] when `value` is empty after trimming.
    /// Returns [`ModelError::InvalidPaneIdFormat`] when `value` is not `%` followed by ASCII digits.
    pub fn new(value: &str) -> Result<Self, ModelError> {
        if contains_disallowed_display_control(value) {
            return Err(ModelError::ControlCharacter);
        }
        if value.trim().is_empty() {
            return Err(ModelError::EmptyId);
        }
        if !is_tmux_pane_id(value) {
            return Err(ModelError::InvalidPaneIdFormat);
        }

        Ok(Self(value.to_owned()))
    }

    /// Return the identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_tmux_pane_id(value: &str) -> bool {
    value.strip_prefix('%').is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}
