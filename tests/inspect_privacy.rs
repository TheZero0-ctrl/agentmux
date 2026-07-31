//! Privacy regressions for inspect errors that cross public caller seams.

use std::cell::RefCell;

use agentmux::inspect::run_inspect;
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput};

const FIELD_SEPARATOR: char = '\u{1f}';
const PRIVATE_PATH: &str = "/home/alice/private-repo";
const SECRET_TOKEN: &str = "token=super-secret";
const SECRET_SESSION: &str = "session-sk-live-secret";
const EXPECTED_LIST_PANES_FORMAT: &str = "#{session_name}\u{1f}#{window_index}\u{1f}#{window_name}\u{1f}#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}";

#[test]
fn given_secret_bearing_malformed_tmux_when_inspect_errors_then_display_and_debug_are_sanitized() {
    // Given: tmux returns a malformed row with private path and token-like values.
    let command = FakeTmuxCommand::new(Ok(tmux_row([
        SECRET_SESSION,
        "0",
        "api",
        "%1",
        "not-a-pid",
        "0",
        PRIVATE_PATH,
        SECRET_TOKEN,
    ])));
    let mut stdout = Vec::new();

    // When: inspect returns its public typed error.
    let error = run_inspect(command, &mut stdout).expect_err("inspect fails");
    let display = error.to_string();
    let debug = format!("{error:?}");

    // Then: no partial stdout or raw tmux data reaches Display or Debug.
    assert!(stdout.is_empty());
    for formatted in [display.as_str(), debug.as_str()] {
        assert!(!formatted.contains(PRIVATE_PATH));
        assert!(!formatted.contains(SECRET_TOKEN));
        assert!(!formatted.contains("not-a-pid"));
        assert!(!formatted.contains(SECRET_SESSION));
    }
}

fn tmux_row(fields: [&str; 8]) -> String {
    format!("{}\n", fields.join(&FIELD_SEPARATOR.to_string()))
}

struct FakeTmuxCommand {
    result: RefCell<Option<Result<String, TmuxError>>>,
}

impl FakeTmuxCommand {
    const fn new(result: Result<String, TmuxError>) -> Self {
        Self {
            result: RefCell::new(Some(result)),
        }
    }
}

impl TmuxCommand for FakeTmuxCommand {
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError> {
        assert_eq!(format, EXPECTED_LIST_PANES_FORMAT);
        self.result
            .borrow_mut()
            .take()
            .ok_or(TmuxError::CommandFailed { status: None })
            .and_then(|output| output.map(TmuxOutput::new))
    }
}
