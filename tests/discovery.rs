//! Phase 2 tracer seam tests for tmux/process discovery.

use std::cell::RefCell;
use std::collections::VecDeque;

use agentmux::daemon::DiscoveryService;
use agentmux::model::{
    AgentId, AgentState, EvidenceConfidence, EvidenceFreshness, EvidenceSource, Pane, PaneId,
    ProcessEvidence,
};
use agentmux::process::classify_process;
use agentmux::state::{AgentSnapshot, normalize_snapshot};
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput, parse_list_panes};

const FIELD_SEPARATOR: char = '\u{1f}';
const EXPECTED_LIST_PANES_FORMAT: &str =
    "#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_command}";

#[test]
fn given_tmux_rows_when_parsed_then_valid_rows_become_panes() {
    // Given: delimiter-separated tmux list-panes rows.
    let output = format!(
        "%1{FIELD_SEPARATOR}123{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n%2{FIELD_SEPARATOR}456{FIELD_SEPARATOR}1{FIELD_SEPARATOR}zsh\n"
    );

    // When: the list-panes output is parsed.
    let panes = parse_list_panes(&output).expect("tmux rows parse");

    // Then: valid rows become typed panes with process evidence.
    assert_eq!(panes.len(), 2);
    let mut panes = panes.iter();
    let first = panes.next().expect("first pane exists");
    let second = panes.next().expect("second pane exists");
    assert_eq!(first.id().as_str(), "%1");
    assert_eq!(first.process().pid(), Some(123));
    assert_eq!(first.process().command(), Some("bash"));
    assert_eq!(second.process().pid(), Some(456));
    assert_eq!(
        second.process().evidence().confidence(),
        EvidenceConfidence::Medium
    );
    assert_eq!(classify_process(second.process()), AgentState::Exited);
}

#[test]
fn given_command_contains_field_separator_when_parsed_then_command_remainder_is_preserved() {
    // Given: tmux emits a command value containing the record field separator.
    let output = format!(
        "%1{FIELD_SEPARATOR}123{FIELD_SEPARATOR}0{FIELD_SEPARATOR}shell{FIELD_SEPARATOR}detail\n"
    );

    // When: the list-panes output is parsed.
    let panes = parse_list_panes(&output).expect("tmux row parses");

    // Then: only the first three separators split fields and the command remainder is intact.
    assert_eq!(panes.len(), 1);
    assert_eq!(
        panes.first().and_then(|pane| pane.process().command()),
        Some("shell\u{1f}detail")
    );
}

#[test]
fn given_invalid_tmux_rows_when_parsed_then_errors_are_typed() {
    // Given: malformed field count, pid, boolean, invalid id, and empty tmux outputs.
    let missing_fields = format!("%1{FIELD_SEPARATOR}123{FIELD_SEPARATOR}0\n");
    let malformed_pid = format!("%1{FIELD_SEPARATOR}abc{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n");
    let malformed_bool =
        format!("%1{FIELD_SEPARATOR}123{FIELD_SEPARATOR}maybe{FIELD_SEPARATOR}bash\n");
    let invalid_pane_id =
        format!("%1\tbad{FIELD_SEPARATOR}123{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n");

    // When: each output is parsed.
    // Then: malformed data is rejected while empty output is a valid empty snapshot.
    assert!(matches!(
        parse_list_panes(&missing_fields),
        Err(TmuxError::MalformedFieldCount { .. })
    ));
    assert!(matches!(
        parse_list_panes(&malformed_pid),
        Err(TmuxError::MalformedPid { .. })
    ));
    assert!(matches!(
        parse_list_panes(&malformed_bool),
        Err(TmuxError::MalformedBoolean { .. })
    ));
    assert!(matches!(
        parse_list_panes(&invalid_pane_id),
        Err(TmuxError::InvalidPaneId { .. })
    ));
    assert_eq!(
        parse_list_panes("").expect("empty output parses"),
        Vec::new()
    );
}

#[test]
fn given_tmux_command_failure_when_refreshed_then_the_error_surfaces() {
    // Given: an injectable tmux command boundary that fails.
    let command = FakeTmuxCommand::new([Err(TmuxError::CommandFailed {
        status: Some(1),
        stderr: String::from("no server running"),
    })]);
    let mut service = DiscoveryService::new(command);

    // When: discovery refreshes from tmux.
    let result = service.refresh();

    // Then: command failure is returned as a typed error.
    assert!(matches!(result, Err(TmuxError::CommandFailed { .. })));
}

#[test]
fn given_pane_snapshot_when_normalized_then_agents_are_deterministic_and_conservative() {
    // Given: unordered fresh, stale, and incomplete pane evidence.
    let panes = vec![
        Pane::new(
            PaneId::new("%3").expect("pane id is valid"),
            ProcessEvidence::incomplete_live(303),
        ),
        Pane::new(
            PaneId::new("%1").expect("pane id is valid"),
            ProcessEvidence::live(101, "bash"),
        ),
        Pane::new(
            PaneId::new("%2").expect("pane id is valid"),
            ProcessEvidence::live(202, "python").into_stale(),
        ),
    ];

    // When: a successful snapshot is normalized.
    let snapshot = normalize_snapshot(&AgentSnapshot::default(), panes);

    // Then: ordering is deterministic and stale/incomplete evidence stays unknown.
    assert_eq!(
        snapshot.agent_ids(),
        vec![
            AgentId::new("pane:%1").expect("agent id is valid"),
            AgentId::new("pane:%2").expect("agent id is valid"),
            AgentId::new("pane:%3").expect("agent id is valid"),
        ]
    );
    assert_eq!(snapshot.agent_state("pane:%1"), Some(AgentState::Idle));
    assert_eq!(snapshot.agent_state("pane:%2"), Some(AgentState::Unknown));
    assert_eq!(snapshot.agent_state("pane:%3"), Some(AgentState::Unknown));
}

#[test]
fn given_previous_pane_missing_when_normalized_then_it_exits_with_missing_evidence() {
    // Given: a previous snapshot with one tracked pane.
    let first = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![Pane::new(
            PaneId::new("%1").expect("pane id is valid"),
            ProcessEvidence::live(101, "bash"),
        )],
    );

    // When: the latest successful snapshot no longer contains that pane.
    let second = normalize_snapshot(&first, Vec::new());
    let missing = second.agent("pane:%1").expect("agent remains tracked");

    // Then: the pane is marked exited with explicit missing-pane evidence.
    assert_eq!(missing.state(), AgentState::Exited);
    assert_eq!(missing.evidence().source(), EvidenceSource::MissingPane);
    assert_eq!(missing.evidence().freshness(), EvidenceFreshness::Fresh);
    assert_eq!(missing.evidence().confidence(), EvidenceConfidence::Medium);
}

#[test]
fn given_discovery_service_when_refreshed_then_it_retains_only_previous_snapshot() {
    // Given: a daemon-owned discovery facade with three fake tmux snapshots.
    let first = format!("%1{FIELD_SEPARATOR}101{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n");
    let command = FakeTmuxCommand::new([Ok(first), Ok(String::new()), Ok(String::new())]);
    let mut service = DiscoveryService::new(command);

    // When: discovery refreshes through live, first missing, and second missing snapshots.
    let initial = service.refresh().expect("first refresh succeeds");
    let next = service.refresh().expect("second refresh succeeds");
    let pruned = service.refresh().expect("third refresh succeeds");

    // Then: live panes become one exited tombstone and already-exited tombstones are pruned.
    assert_eq!(initial.agent_state("pane:%1"), Some(AgentState::Idle));
    assert_eq!(next.agent_state("pane:%1"), Some(AgentState::Exited));
    assert_eq!(pruned.agent_state("pane:%1"), None);
}

#[test]
fn given_failed_refresh_between_snapshots_when_refreshed_then_last_success_is_retained() {
    // Given: a successful snapshot, a tmux failure, and a later empty snapshot.
    let first = format!("%1{FIELD_SEPARATOR}101{FIELD_SEPARATOR}0{FIELD_SEPARATOR}bash\n");
    let command = FakeTmuxCommand::new([
        Ok(first),
        Err(TmuxError::CommandFailed {
            status: Some(1),
            stderr: String::from("temporary tmux failure"),
        }),
        Ok(String::new()),
    ]);
    let mut service = DiscoveryService::new(command);

    // When: the middle refresh fails before the later successful refresh.
    let initial = service.refresh().expect("first refresh succeeds");
    let failure = service.refresh();
    let recovered = service.refresh().expect("third refresh succeeds");

    // Then: the failure does not erase the last successful snapshot.
    assert_eq!(initial.agent_state("pane:%1"), Some(AgentState::Idle));
    assert!(matches!(failure, Err(TmuxError::CommandFailed { .. })));
    assert_eq!(recovered.agent_state("pane:%1"), Some(AgentState::Exited));
}

struct FakeTmuxCommand {
    outputs: RefCell<VecDeque<Result<String, TmuxError>>>,
}

impl FakeTmuxCommand {
    fn new<const N: usize>(outputs: [Result<String, TmuxError>; N]) -> Self {
        Self {
            outputs: RefCell::new(VecDeque::from(outputs)),
        }
    }
}

impl TmuxCommand for FakeTmuxCommand {
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError> {
        assert_eq!(format, EXPECTED_LIST_PANES_FORMAT);

        self.outputs
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| TmuxError::CommandFailed {
                status: None,
                stderr: String::from("fake tmux command exhausted before test completed"),
            })
            .and_then(|output| output.map(TmuxOutput::new))
    }
}
