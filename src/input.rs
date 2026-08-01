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
    /// The Up arrow key.
    Up,
    /// The Down arrow key.
    Down,
    /// The Home key.
    Home,
    /// The End key.
    End,
    /// The `PageUp` key.
    PageUp,
    /// The `PageDown` key.
    PageDown,
    /// Any other key.
    Other,
}

/// Translate a dashboard key input into an application action.
pub const fn handle_key(input: KeyInput) -> Option<Action> {
    match input {
        KeyInput::Character('q') | KeyInput::Escape | KeyInput::ControlC => Some(Action::Quit),
        KeyInput::Character('r' | 'R') => Some(Action::Refresh),
        KeyInput::Character('j') | KeyInput::Down => Some(Action::SelectNext),
        KeyInput::Character('k') | KeyInput::Up => Some(Action::SelectPrevious),
        KeyInput::Home => Some(Action::SelectFirst),
        KeyInput::End => Some(Action::SelectLast),
        KeyInput::PageDown => Some(Action::PageNext),
        KeyInput::PageUp => Some(Action::PagePrevious),
        KeyInput::Character('?' | 'h') => Some(Action::ToggleHelp),
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
    fn given_lowercase_r_when_handled_then_it_refreshes() {
        // Given: a manual refresh key input.
        // When: the key is translated.
        // Then: the dashboard should refresh immediately.
        let action = handle_key(KeyInput::Character('r'));
        assert_eq!(action, Some(Action::Refresh));
    }

    #[test]
    fn given_uppercase_r_when_handled_then_it_refreshes() {
        // Given: an uppercase manual refresh key input.
        // When: the key is translated.
        // Then: the dashboard should refresh immediately.
        let action = handle_key(KeyInput::Character('R'));
        assert_eq!(action, Some(Action::Refresh));
    }

    #[test]
    fn given_other_key_when_handled_then_it_does_not_quit() {
        // Given: a non-quit key input.
        // When: the key is translated.
        // Then: the dashboard should keep running.
        let action = handle_key(KeyInput::Other);
        assert_eq!(action, None);
    }

    #[test]
    fn given_navigation_keys_when_handled_then_selection_actions_are_returned() {
        // Given: dashboard navigation key inputs.
        // When: each key is translated.
        // Then: selection actions are returned.
        assert_eq!(handle_key(KeyInput::Down), Some(Action::SelectNext));
        assert_eq!(
            handle_key(KeyInput::Character('j')),
            Some(Action::SelectNext)
        );
        assert_eq!(handle_key(KeyInput::Up), Some(Action::SelectPrevious));
        assert_eq!(
            handle_key(KeyInput::Character('k')),
            Some(Action::SelectPrevious)
        );
        assert_eq!(handle_key(KeyInput::Home), Some(Action::SelectFirst));
        assert_eq!(handle_key(KeyInput::End), Some(Action::SelectLast));
        assert_eq!(handle_key(KeyInput::PageDown), Some(Action::PageNext));
        assert_eq!(handle_key(KeyInput::PageUp), Some(Action::PagePrevious));
    }

    #[test]
    fn given_help_keys_when_handled_then_toggle_help_is_returned() {
        // Given: help key inputs.
        // When: each key is translated.
        // Then: the help action is returned.
        assert_eq!(
            handle_key(KeyInput::Character('?')),
            Some(Action::ToggleHelp)
        );
        assert_eq!(
            handle_key(KeyInput::Character('h')),
            Some(Action::ToggleHelp)
        );
    }
}
