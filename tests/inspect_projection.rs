//! Enriched inspect projection privacy contracts.

use std::cell::RefCell;
use std::path::Path;

use agentmux::inspect::run_inspect;
use agentmux::model::{
    ClientCandidate, ClientConfidence, ClientKind, LocationMetadata, ModelError, Pane, PaneId,
    ProcessBasename, ProcessEvidence, ProcessIdentity, ProcessMetadata, SessionName, SessionWindow,
    WindowIndex, WindowName, WorkspaceLabel,
};
use agentmux::projection::project_snapshot;
use agentmux::state::{AgentSnapshot, normalize_snapshot};
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput};

const FIELD_SEPARATOR: char = '\u{1f}';
const EXPECTED_LIST_PANES_FORMAT: &str = "#{session_name}\u{1f}#{window_index}\u{1f}#{window_name}\u{1f}#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}";
const EXPECTED_INSPECT_HEADER: &str = "agent_id\tsession_name\twindow_index\twindow_name\tpane_id\tpid\tprocess_name\tclient\tclient_confidence\tworkspace\tstate\tevidence_source\tevidence_freshness\tevidence_confidence";

#[test]
fn given_enriched_snapshot_when_projected_then_complete_privacy_safe_row_is_returned()
-> Result<(), ModelError> {
    // Given: a normalized snapshot with typed location and selected process metadata.
    let snapshot = normalize_snapshot(
        &AgentSnapshot::default(),
        [pane_with_metadata(
            "%7",
            ProcessMetadata::new(
                Some(ProcessIdentity::new(777, 12_345)),
                Some(ProcessBasename::new("codex")?),
                ClientCandidate::known(ClientKind::Codex, ClientConfidence::Low),
            ),
        )?],
    );

    // When: the shared projection creates inspect/dashboard rows.
    let rows = project_snapshot(&snapshot);

    // Then: every Todo 9 inspect column has a privacy-safe value.
    assert_eq!(rows.len(), 1);
    let row = rows.first().expect("one row");
    assert_eq!(row.agent_id(), "pane:%7");
    assert_eq!(row.session_name(), "unknown");
    assert_eq!(row.window_index(), "3");
    assert_eq!(row.window_name(), "unknown");
    assert_eq!(row.pane_id(), "%7");
    assert_eq!(row.pid(), "777");
    assert_eq!(row.process_name(), "codex");
    assert_eq!(row.client(), "codex");
    assert_eq!(row.client_confidence(), "low");
    assert_eq!(row.workspace(), "project");
    assert_eq!(row.state(), "working");
    assert_eq!(row.evidence_source(), "process");
    assert_eq!(row.evidence_freshness(), "fresh");
    assert_eq!(row.evidence_confidence(), "low");
    Ok(())
}

#[test]
fn given_sensitive_tmux_values_when_inspected_then_output_uses_safe_labels() {
    // Given: successful tmux fields contain sensitive-looking session, window, and workspace values.
    let secret_path = "/home/alice/private-repo";
    let prompt_like_session = "prompt says token sk-live-secret";
    let prompt_like_window = "diff --git raw stderr";
    let tmux_output = format!(
        "{}\n",
        tmux_row([
            prompt_like_session,
            "0",
            prompt_like_window,
            "%1",
            "101",
            "0",
            secret_path,
            "bash",
        ]),
    );
    let command = FakeTmuxCommand::new(Ok(tmux_output));
    let mut stdout = Vec::new();

    // When: inspect writes its successful row.
    run_inspect(command, &mut stdout).expect("inspect succeeds");

    // Then: stdout never exposes the full private cwd and still has the exact enriched schema.
    let stdout = String::from_utf8(stdout).expect("stdout is utf8");
    assert!(stdout.contains(EXPECTED_INSPECT_HEADER));
    assert!(stdout.contains("pane:%1\tunknown\t0\tunknown\t%1"));
    assert!(stdout.contains("\tprivate-repo\t"));
    assert!(!stdout.contains(secret_path));
    assert!(!stdout.contains("/home/alice"));
    assert!(!stdout.contains("prompt"));
    assert!(!stdout.contains("sk-live-secret"));
    assert!(!stdout.contains("diff --git"));
    assert!(!stdout.contains("raw stderr"));
    assert!(
        stdout
            .lines()
            .all(|line| !line.contains('\t') || line.split('\t').count() == 14)
    );
}

#[test]
fn given_secret_bearing_tmux_failures_when_inspected_then_errors_do_not_leak_raw_values() {
    // Given: tmux failures include path, prompt, token, tab, newline, diff, and raw stderr text.
    let secret = "/home/alice/private-repo prompt token=sk-live-secret\tdiff --git\nraw stderr";
    let cases = [
        Err(TmuxError::command_failed(Some(2), secret)),
        Ok(format!(
            "{}\n",
            tmux_row(["work", "0", "api", "%1", "101", "0", secret, "bash"]),
        )),
    ];

    // When: inspect fails before writing stdout.
    // Then: Display and Debug expose only typed sanitized metadata.
    for result in cases {
        let command = FakeTmuxCommand::new(result);
        let mut stdout = Vec::new();
        let error = run_inspect(command, &mut stdout).expect_err("inspect fails");
        let display = error.to_string();
        let debug = format!("{error:?}");

        assert!(stdout.is_empty());
        for rendered in [display, debug] {
            assert!(!rendered.contains("/home/alice/private-repo"));
            assert!(!rendered.contains("sk-live-secret"));
            assert!(!rendered.contains("diff --git"));
            assert!(!rendered.contains("raw stderr"));
            assert!(!rendered.contains(secret));
        }
    }
}

fn tmux_row(fields: [&str; 8]) -> String {
    fields.join(&FIELD_SEPARATOR.to_string())
}

fn pane_with_metadata(
    pane_id: &str,
    process_metadata: ProcessMetadata,
) -> Result<Pane, ModelError> {
    Ok(Pane::with_location(
        LocationMetadata::new(
            PaneId::new(pane_id)?,
            SessionWindow::new(
                SessionName::new("work")?,
                WindowIndex::new(3),
                WindowName::new("editor")?,
            ),
            WorkspaceLabel::from_path(Path::new("/home/alice/project"))?,
        ),
        ProcessEvidence::live(777, "codex"),
    )
    .with_process_metadata(process_metadata))
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
