//! Public seam tests for one-shot inspect output.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use agentmux::inspect::{InspectError, run_inspect};
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput};

const FIELD_SEPARATOR: char = '\u{1f}';
const EXPECTED_LIST_PANES_FORMAT: &str =
    "#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_command}";

#[test]
fn given_reversed_tmux_rows_when_inspected_then_output_is_deterministically_sorted() {
    // Given: tmux returns panes in reverse display order through the injected seam.
    let tmux_output = format!(
        "%2{FIELD_SEPARATOR}202{FIELD_SEPARATOR}0{FIELD_SEPARATOR}python\n%1{FIELD_SEPARATOR}101{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n"
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let calls = command.calls();
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: stdout is stable, sorted by agent id, and tmux was called once.
    assert_eq!(calls.borrow().len(), 1);
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\n\
agents: 2\n\
agent_id\tpane_id\tstate\tevidence_source\tevidence_freshness\tevidence_confidence\n\
pane:%1\t%1\tidle\ttmux\tfresh\tlow\n\
pane:%2\t%2\tworking\ttmux\tfresh\tlow\n"
    );
}

#[test]
fn given_multi_digit_pane_ids_when_inspected_then_output_is_sorted_by_pane_number() {
    // Given: tmux returns multi-digit pane IDs before single-digit pane IDs.
    let tmux_output = format!(
        "%10{FIELD_SEPARATOR}1000{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%2{FIELD_SEPARATOR}202{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%1{FIELD_SEPARATOR}101{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n"
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: rows render in numeric pane order, not lexicographic agent-id order.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\n\
agents: 3\n\
agent_id\tpane_id\tstate\tevidence_source\tevidence_freshness\tevidence_confidence\n\
pane:%1\t%1\tidle\ttmux\tfresh\tlow\n\
pane:%2\t%2\tidle\ttmux\tfresh\tlow\n\
pane:%10\t%10\tidle\ttmux\tfresh\tlow\n"
    );
}

#[test]
fn given_leading_zero_pane_ids_when_inspected_then_output_is_sorted_by_numeric_magnitude() {
    // Given: tmux returns leading-zero pane IDs around canonical forms.
    let tmux_output = format!(
        "%10{FIELD_SEPARATOR}1000{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%0002{FIELD_SEPARATOR}200{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%2{FIELD_SEPARATOR}202{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%0000{FIELD_SEPARATOR}400{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%0{FIELD_SEPARATOR}401{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n"
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: significant digits define magnitude, with raw suffix as deterministic tie-breaker.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\n\
agents: 5\n\
agent_id\tpane_id\tstate\tevidence_source\tevidence_freshness\tevidence_confidence\n\
pane:%0\t%0\tidle\ttmux\tfresh\tlow\n\
pane:%0000\t%0000\tidle\ttmux\tfresh\tlow\n\
pane:%2\t%2\tidle\ttmux\tfresh\tlow\n\
pane:%0002\t%0002\tidle\ttmux\tfresh\tlow\n\
pane:%10\t%10\tidle\ttmux\tfresh\tlow\n"
    );
}

#[test]
fn given_dead_tmux_row_when_inspected_then_dead_evidence_is_medium_confidence() {
    // Given: tmux returns a dead pane through the injected seam.
    let tmux_output = format!("%1{FIELD_SEPARATOR}101{FIELD_SEPARATOR}1{FIELD_SEPARATOR}bash\n");
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: stdout reports exited state with medium-confidence tmux evidence.
    assert_eq!(
        String::from_utf8(stdout).expect("stdout is utf8"),
        "agentmux inspect\n\
agents: 1\n\
agent_id\tpane_id\tstate\tevidence_source\tevidence_freshness\tevidence_confidence\n\
pane:%1\t%1\texited\ttmux\tfresh\tmedium\n"
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
    let command = FakeTmuxCommand::new(Err(TmuxError::CommandFailed {
        status: Some(1),
        stderr: String::from("no server running"),
    }));
    let mut stdout = Vec::new();

    // When: inspect runs once.
    let result = run_inspect(command, &mut stdout);

    // Then: the tmux error is wrapped and stdout remains untouched.
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
            .ok_or_else(|| TmuxError::CommandFailed {
                status: None,
                stderr: String::from("fake tmux command exhausted before test completed"),
            })
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
