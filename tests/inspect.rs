//! Public seam tests for one-shot inspect output.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use agentmux::inspect::{InspectError, run_inspect};
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput};

const FIELD_SEPARATOR: char = '\u{1f}';
const EXPECTED_LIST_PANES_FORMAT: &str = "#{session_name}\u{1f}#{window_index}\u{1f}#{window_name}\u{1f}#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}";
const NON_ENRICHING_PID: &str = "4294967295";

fn tmux_row(fields: [&str; 8]) -> String {
    fields.join(&FIELD_SEPARATOR.to_string())
}

fn live_row(
    window_index: &str,
    window_name: &str,
    pane_id: &str,
    workspace: &str,
    command: &str,
) -> String {
    tmux_row([
        "work",
        window_index,
        window_name,
        pane_id,
        NON_ENRICHING_PID,
        "0",
        workspace,
        command,
    ])
}

#[test]
fn given_reversed_tmux_rows_when_inspected_then_non_agent_rows_are_filtered() {
    // Given: tmux returns panes in reverse display order through the injected seam.
    let tmux_output = format!(
        "{}\n{}\n",
        live_row("1", "api", "%2", "/tmp/api", "codex"),
        live_row("0", "shell", "%1", "/tmp/shell", "codex"),
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let calls = command.calls();
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: stdout omits tmux-only non-agent panes, and tmux was called once.
    assert_eq!(calls.borrow().len(), 1);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\nagents: 0\nno agents discovered from tmux panes\n"
    );
}

#[test]
fn given_multi_digit_pane_ids_when_inspected_then_non_agent_rows_are_filtered() {
    // Given: tmux returns multi-digit pane IDs before single-digit pane IDs.
    let tmux_output = format!(
        "{}\n{}\n{}\n",
        live_row("0", "ten", "%10", "/tmp/ten", "codex"),
        live_row("9", "two", "%2", "/tmp/two", "codex"),
        live_row("5", "one", "%1", "/tmp/one", "codex"),
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: tmux-only non-agent panes are omitted before presentation.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\nagents: 0\nno agents discovered from tmux panes\n"
    );
}

#[test]
fn given_leading_zero_pane_ids_when_inspected_then_non_agent_rows_are_filtered() {
    // Given: tmux returns leading-zero pane IDs around canonical forms.
    let tmux_output = format!(
        "{}\n{}\n{}\n{}\n{}\n",
        live_row("10", "ten", "%10", "/tmp/ten", "codex"),
        live_row("2", "two-a", "%0002", "/tmp/two-a", "codex"),
        live_row("2", "two-b", "%2", "/tmp/two-b", "codex"),
        live_row("0", "zero-a", "%0000", "/tmp/zero-a", "codex"),
        live_row("0", "zero-b", "%0", "/tmp/zero-b", "codex"),
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: tmux-only non-agent panes are omitted before presentation.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\nagents: 0\nno agents discovered from tmux panes\n"
    );
}

#[test]
fn given_dead_tmux_row_when_inspected_then_dead_evidence_is_medium_confidence() {
    // Given: tmux returns a dead pane through the injected seam.
    let tmux_output = format!(
        "{}\n",
        tmux_row(["work", "0", "api", "%1", "101", "1", "/tmp/api", "bash"])
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: dead non-agent panes are omitted from presentation output.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\nagents: 0\nno agents discovered from tmux panes\n"
    );
}

#[test]
fn given_empty_tmux_snapshot_when_inspected_then_output_explains_no_agents() {
    // Given: tmux returns no panes through the injected seam.
    let command = FakeTmuxCommand::new(Ok(String::new()));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: stdout uses the exact empty inspect format.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\nagents: 0\nno agents discovered from tmux panes\n"
    );
}

#[test]
fn given_tmux_failure_when_inspected_then_error_is_typed_and_stdout_stays_empty() {
    // Given: discovery fails before any inspect output should be written.
    let command = FakeTmuxCommand::new(Err(TmuxError::CommandFailed { status: Some(1) }));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    let result = run_inspect(command, &mut stdout);

    // Then: the tmux error is wrapped and stdout remains untouched.
    assert!(matches!(result, Err(InspectError::Tmux { .. })));
    assert!(stdout.is_empty());
}

#[test]
fn given_malformed_tmux_row_when_inspected_then_error_is_typed_and_stdout_stays_empty() {
    // Given: discovery parses an invalid tmux row before inspect writes its preamble.
    let tmux_output = format!(
        "{}\n",
        tmux_row([
            "work",
            "0",
            "api",
            "%1",
            "not-a-pid",
            "0",
            "/tmp/api",
            "bash"
        ])
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    let result = run_inspect(command, &mut stdout);

    // Then: the parse error is wrapped and stdout remains untouched.
    assert!(matches!(result, Err(InspectError::Tmux { .. })));
    assert!(stdout.is_empty());
}

#[test]
fn given_broken_pipe_writer_after_discovery_when_inspected_then_error_is_typed_io() {
    // Given: discovery succeeds but the caller-provided writer hits BrokenPipe.
    let command = FakeTmuxCommand::new(Ok(String::new()));
    let mut writer = BrokenPipeWriter;

    // When: inspect writes its successful snapshot output.
    let result = run_inspect(command, &mut writer);

    // Then: reusable inspect preserves BrokenPipe as an I/O error.
    assert!(matches!(
        result,
        Err(InspectError::Io { source }) if source.kind() == io::ErrorKind::BrokenPipe
    ));
}

struct FakeTmuxCommand {
    result: RefCell<Option<Result<String, TmuxError>>>,
    calls: Rc<RefCell<Vec<()>>>,
}

impl FakeTmuxCommand {
    fn new(result: Result<String, TmuxError>) -> Self {
        Self {
            result: RefCell::new(Some(result)),
            calls: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn calls(&self) -> Rc<RefCell<Vec<()>>> {
        Rc::clone(&self.calls)
    }
}

impl TmuxCommand for FakeTmuxCommand {
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError> {
        assert_eq!(format, EXPECTED_LIST_PANES_FORMAT);
        self.calls.borrow_mut().push(());

        self.result
            .borrow_mut()
            .take()
            .ok_or(TmuxError::CommandFailed { status: None })
            .and_then(|output| output.map(TmuxOutput::new))
    }
}

struct BrokenPipeWriter;

impl io::Write for BrokenPipeWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
