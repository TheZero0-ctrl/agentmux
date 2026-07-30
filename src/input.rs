//! Key input translation for the agentmux dashboard.

use crate::app::Action;

/// Normalized dashboard key inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum KeyInput {
    /// The `q` key.
    Character(char),
    /// The Escape key.
    Escape,
    /// The Ctrl-C key chord.
    ControlC,
    /// Any other key.
    Other,
}

/// Translate a dashboard key input into an application action.
pub const fn handle_key(input: KeyInput) -> Option<Action> {
    match input {
        KeyInput::Character('q') | KeyInput::Escape | KeyInput::ControlC => Some(Action::Quit),
        KeyInput::Character(_) | KeyInput::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::app::Action;

    use super::{KeyInput, handle_key};

    #[test]
    fn given_q_when_handled_then_it_quits() {
        // Given: a quit key input.
        // When: the key is translated.
        // Then: the dashboard should exit.
        let action = handle_key(KeyInput::Character('q'));
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn given_escape_when_handled_then_it_quits() {
        // Given: an escape key input.
        // When: the key is translated.
        // Then: the dashboard should exit.
        let action = handle_key(KeyInput::Escape);
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn given_ctrl_c_when_handled_then_it_quits() {
        // Given: a ctrl-c key input.
        // When: the key is translated.
        // Then: the dashboard should exit.
        let action = handle_key(KeyInput::ControlC);
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn given_other_key_when_handled_then_it_does_not_quit() {
        // Given: a non-quit key input.
        // When: the key is translated.
        // Then: the dashboard should keep running.
        let action = handle_key(KeyInput::Other);
        assert_eq!(action, None);
    }
}
