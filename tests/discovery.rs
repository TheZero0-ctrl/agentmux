//! Phase 2 tracer seam tests for tmux/process discovery.

#[path = "discovery/reconciliation.rs"]
mod reconciliation;
#[path = "discovery/reconciliation_service.rs"]
mod reconciliation_service;
#[path = "discovery/service_integration.rs"]
mod service_integration;
#[path = "discovery/tmux_contracts.rs"]
mod tmux_contracts;
#[path = "discovery/tmux_privacy_contracts.rs"]
mod tmux_privacy_contracts;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;

use agentmux::daemon::DiscoveryService;
use agentmux::model::{
    AgentId, AgentState, EvidenceConfidence, EvidenceFreshness, EvidenceSource, Pane, PaneId,
    ProcessEvidence,
};
use agentmux::state::{AgentSnapshot, normalize_snapshot};
use agentmux::tmux::{TmuxCommand, TmuxError, TmuxOutput};

const FIELD_SEPARATOR: char = '\u{1f}';
const EXPECTED_LIST_PANES_FORMAT: &str = "#{session_name}\u{1f}#{window_index}\u{1f}#{window_name}\u{1f}#{pane_id}\u{1f}#{pane_pid}\u{1f}#{pane_dead}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}";

fn tmux_row(fields: [&str; 8]) -> String {
    fields.join(&FIELD_SEPARATOR.to_string())
}

#[test]
fn given_tmux_command_failure_when_refreshed_then_the_error_surfaces() {
    // Given: an injectable tmux command boundary that fails.
    let command = FakeTmuxCommand::new([Err(TmuxError::CommandFailed { status: Some(1) })]);
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
            AgentId::from_pane_id(&PaneId::new("%1").expect("pane id is valid")),
            AgentId::from_pane_id(&PaneId::new("%2").expect("pane id is valid")),
            AgentId::from_pane_id(&PaneId::new("%3").expect("pane id is valid")),
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
    let first = format!(
        "{}\n",
        tmux_row(["work", "0", "api", "%1", "101", "0", "/tmp", "bash"])
    );
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
    let first = format!(
        "{}\n",
        tmux_row(["work", "0", "api", "%1", "101", "0", "/tmp", "bash"])
    );
    let command = FakeTmuxCommand::new([
        Ok(first),
        Err(TmuxError::CommandFailed { status: Some(1) }),
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

        self.outputs.borrow_mut().pop_front().map_or_else(
            || {
                Err(TmuxError::CommandIo {
                    source: io::Error::other("fake tmux command exhausted"),
                })
            },
            |output| output.map(TmuxOutput::new),
        )
    }
}
