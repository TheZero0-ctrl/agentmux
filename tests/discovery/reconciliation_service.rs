use std::collections::BTreeMap;

use agentmux::daemon::{DiscoveryService, ProcessTreeEvidence};
use agentmux::model::{
    AgentState, ClientCandidate, ClientConfidence, ClientKind, ModelError, ProcessBasename,
    ProcessIdentity,
};
use agentmux::process::procfs::ProcessRecord;
use agentmux::tmux::TmuxError;

use super::{FakeTmuxCommand, tmux_row};

#[test]
fn reconciliation_given_failed_refresh_after_candidate_when_refreshed_then_normalization_is_not_invoked()
-> Result<(), ModelError> {
    // Given: a candidate snapshot followed by a failed tmux refresh.
    let first = format!(
        "{}\n",
        tmux_row([
            "work",
            "0",
            "api",
            "%1",
            "101",
            "0",
            "/tmp/private-api",
            "bash",
        ])
    );
    let command =
        FakeTmuxCommand::new([Ok(first), Err(TmuxError::CommandFailed { status: Some(1) })]);
    let process_source =
        FakeProcessTreeSource::new([(101, [record(101, 1_001, "opencode")?].into())]);
    let mut service = DiscoveryService::with_sources(command, process_source);
    let initial = service.refresh().expect("first refresh succeeds");

    // When: the next refresh fails before panes can be normalized.
    let failure = service.refresh();

    // Then: the retained snapshot is exactly the last success, not a missing-pane tombstone.
    assert!(matches!(failure, Err(TmuxError::CommandFailed { .. })));
    assert_eq!(service.snapshot(), &initial);
    assert_eq!(
        service
            .snapshot()
            .agent("pane:%1")
            .expect("previous candidate remains present")
            .observation()
            .client_candidate(),
        ClientCandidate::known(ClientKind::OpenCode, ClientConfidence::Low)
    );
    assert_eq!(
        service.snapshot().agent_state("pane:%1"),
        Some(AgentState::Idle)
    );
    assert!(service.snapshot().agent("pane:%1").is_some());
    Ok(())
}

#[derive(Debug)]
struct FakeProcessTreeSource {
    trees: BTreeMap<u32, Vec<ProcessRecord>>,
}

impl FakeProcessTreeSource {
    fn new<const N: usize>(trees: [(u32, Vec<ProcessRecord>); N]) -> Self {
        Self {
            trees: BTreeMap::from(trees),
        }
    }
}

impl ProcessTreeEvidence for FakeProcessTreeSource {
    fn read_tree(&self, root_pid: u32) -> Vec<ProcessRecord> {
        self.trees
            .get(&root_pid)
            .map_or_else(Vec::new, Clone::clone)
    }
}

fn record(pid: u32, start_time_ticks: u64, basename: &str) -> Result<ProcessRecord, ModelError> {
    Ok(ProcessRecord::new(
        ProcessIdentity::new(pid, start_time_ticks),
        Some(ProcessBasename::new(basename)?),
    ))
}
