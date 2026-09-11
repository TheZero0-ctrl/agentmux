use std::collections::BTreeMap;

use agentmux::daemon::{DiscoveryService, ProcessTreeEvidence};
use agentmux::model::{
    AgentState, ClientCandidate, ClientConfidence, ClientKind, ModelError, ProcessBasename,
    ProcessIdentity,
};
use agentmux::process::procfs::ProcessRecord;
use agentmux::tmux::{SystemTmuxCommand, TmuxError};

use super::{FakeTmuxCommand, tmux_row};

#[test]
fn given_enriched_tmux_and_process_sources_when_refreshed_then_snapshot_contains_candidate_and_unknown_rows()
-> Result<(), ModelError> {
    // Given: two tmux panes and injected process trees where one pane has an exact client candidate.
    let tmux_output = format!(
        "{}\n{}\n",
        tmux_row(["w", "0", "a", "%1", "101", "0", "/t/a", "opencode"]),
        tmux_row(["w", "1", "o", "%2", "202", "0", "/t/o", "zsh"]),
    );
    let process_source = FakeProcessTreeSource::new([
        (202, vec![record(202, 2_002, "zsh")?]),
        (
            101,
            vec![record(101, 1_001, "bash")?, record(111, 1_111, "opencode")?],
        ),
    ]);
    let command = FakeTmuxCommand::new([Ok(tmux_output)]);
    let mut service = DiscoveryService::with_sources(command, process_source);

    // When: discovery refreshes through the injected tmux and process seams.
    let snapshot = service.refresh().expect("refresh succeeds");
    let candidate = snapshot
        .agent("pane:%1")
        .expect("candidate pane is present");
    let unknown = snapshot.agent("pane:%2").expect("unknown pane is present");

    // Then: candidate metadata reaches the normalized snapshot while unsupported trees stay unknown.
    assert_eq!(candidate.state(), AgentState::Working);
    assert_eq!(
        candidate.observation().process_identity(),
        Some(&ProcessIdentity::new(111, 1_111))
    );
    assert_eq!(
        candidate.observation().process_basename(),
        Some(&ProcessBasename::new("opencode")?)
    );
    assert_eq!(
        candidate.observation().client_candidate(),
        ClientCandidate::known(ClientKind::OpenCode, ClientConfidence::Low)
    );
    assert_eq!(unknown.state(), AgentState::Idle);
    assert_eq!(unknown.observation().process_identity(), None);
    assert_eq!(unknown.observation().process_basename(), None);
    assert_eq!(
        unknown.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    Ok(())
}

#[test]
fn given_shell_with_background_agent_processes_when_refreshed_then_panes_stay_unknown()
-> Result<(), ModelError> {
    // A helper/server process must not make an otherwise ordinary shell look like an agent pane.
    let tmux_output = format!(
        "{}\n{}\n{}\n{}\n",
        tmux_row(["w", "0", "a", "%1", "101", "0", "/t/a", "bash"]),
        tmux_row(["w", "1", "b", "%2", "202", "0", "/t/b", "bash"]),
        tmux_row(["w", "2", "c", "%3", "303", "0", "/t/c", "bash"]),
        tmux_row(["w", "3", "d", "%4", "404", "0", "/t/d", "bash"]),
    );
    let process_source = FakeProcessTreeSource::new([
        (
            101,
            vec![record(101, 1_001, "bash")?, record(111, 1_111, "opencode")?],
        ),
        (
            202,
            vec![record(202, 2_002, "bash")?, record(222, 2_222, "codex")?],
        ),
        (
            303,
            vec![record(303, 3_003, "bash")?, record(333, 3_333, "claude")?],
        ),
        (
            404,
            vec![record(404, 4_004, "bash")?, record(444, 4_444, "gemini")?],
        ),
    ]);
    let command = FakeTmuxCommand::new([Ok(tmux_output)]);
    let mut service = DiscoveryService::with_sources(command, process_source);

    let snapshot = service.refresh().expect("refresh succeeds");
    for pane_id in ["pane:%1", "pane:%2", "pane:%3", "pane:%4"] {
        let pane = snapshot.agent(pane_id).expect("pane is present");
        assert_eq!(
            pane.observation().client_candidate(),
            ClientCandidate::Unknown
        );
        assert_eq!(pane.state(), AgentState::Idle);
    }
    Ok(())
}

#[test]
fn given_agent_command_with_arguments_when_refreshed_then_all_agents_are_detected()
-> Result<(), ModelError> {
    let tmux_output = format!(
        "{}\n{}\n{}\n{}\n",
        tmux_row([
            "w",
            "0",
            "a",
            "%1",
            "101",
            "0",
            "/t/a",
            "opencode --continue"
        ]),
        tmux_row(["w", "1", "b", "%2", "202", "0", "/t/b", "codex --full-auto"]),
        tmux_row(["w", "2", "c", "%3", "303", "0", "/t/c", "claude --continue"]),
        tmux_row(["w", "3", "d", "%4", "404", "0", "/t/d", "gemini --resume"]),
    );
    let process_source = FakeProcessTreeSource::new([
        (101, vec![record(101, 1_001, "opencode")?]),
        (202, vec![record(202, 2_002, "codex")?]),
        (303, vec![record(303, 3_003, "claude")?]),
        (404, vec![record(404, 4_004, "gemini")?]),
    ]);
    let command = FakeTmuxCommand::new([Ok(tmux_output)]);
    let mut service = DiscoveryService::with_sources(command, process_source);

    let snapshot = service.refresh().expect("refresh succeeds");
    for (pane_id, kind) in [
        ("pane:%1", ClientKind::OpenCode),
        ("pane:%2", ClientKind::Codex),
        ("pane:%3", ClientKind::Claude),
        ("pane:%4", ClientKind::Gemini),
    ] {
        assert_eq!(
            snapshot
                .agent(pane_id)
                .expect("pane is present")
                .observation()
                .client_candidate(),
            ClientCandidate::known(kind, ClientConfidence::Low)
        );
    }
    Ok(())
}

#[test]
fn given_one_pane_process_tree_degrades_when_refreshed_then_other_panes_still_keep_evidence()
-> Result<(), ModelError> {
    // Given: two panes where the first process tree represents a procfs permission/race degradation.
    let tmux_output = format!(
        "{}\n{}\n",
        tmux_row(["w", "0", "a", "%1", "101", "0", "/t/a", "bash"]),
        tmux_row(["w", "1", "o", "%2", "202", "0", "/t/o", "codex"]),
    );
    let process_source = FakeProcessTreeSource::new([
        (
            202,
            vec![record(202, 2_002, "codex")?, record(222, 2_222, "codex")?],
        ),
        (101, Vec::new()),
    ]);
    let command = FakeTmuxCommand::new([Ok(tmux_output)]);
    let mut service = DiscoveryService::with_sources(command, process_source);

    // When: discovery refreshes every pane independently.
    let snapshot = service.refresh().expect("refresh succeeds");
    let degraded = snapshot.agent("pane:%1").expect("degraded pane is present");
    let candidate = snapshot
        .agent("pane:%2")
        .expect("candidate pane is present");

    // Then: the degraded pane is unknown and the other pane is not suppressed.
    assert_eq!(
        degraded.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    assert_eq!(degraded.observation().process_identity(), None);
    assert_eq!(
        candidate.observation().client_candidate(),
        ClientCandidate::known(ClientKind::Codex, ClientConfidence::Low)
    );
    assert_eq!(
        candidate.observation().process_identity(),
        Some(&ProcessIdentity::new(202, 2_002))
    );
    Ok(())
}

#[test]
fn given_dead_pane_with_client_like_process_tree_when_refreshed_then_process_metadata_stays_unknown()
-> Result<(), ModelError> {
    // Given: a dead tmux pane whose stale PID would otherwise look like a supported client.
    let tmux_output = format!(
        "{}\n",
        tmux_row(["work", "0", "api", "%1", "101", "1", "/tmp", "bash"]),
    );
    let process_source = FakeProcessTreeSource::new([(
        101,
        vec![record(101, 1_001, "bash")?, record(111, 1_111, "opencode")?],
    )]);
    let command = FakeTmuxCommand::new([Ok(tmux_output)]);
    let mut service = DiscoveryService::with_sources(command, process_source);

    // When: discovery refreshes the dead pane.
    let snapshot = service.refresh().expect("refresh succeeds");
    let agent = snapshot.agent("pane:%1").expect("dead pane is present");

    // Then: dead pane PIDs are not enriched with process-tree metadata.
    assert_eq!(agent.state(), AgentState::Exited);
    assert_eq!(agent.observation().process_identity(), None);
    assert_eq!(agent.observation().process_basename(), None);
    assert_eq!(
        agent.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    Ok(())
}

#[test]
fn given_privacy_sensitive_tmux_parse_failure_when_refreshed_then_previous_snapshot_is_untouched() {
    // Given: one successful snapshot followed by a malformed secret-bearing tmux row.
    let first = format!(
        "{}\n",
        tmux_row(["w", "0", "a", "%1", "101", "0", "/t/a", "bash"])
    );
    let secret = "/home/alice/private-repo token=super-secret";
    let malformed = format!(
        "{}\n",
        tmux_row(["w", "0", "a", "%2", "not-a-pid", "0", secret, "opencode"])
    );
    let command = FakeTmuxCommand::new([Ok(first), Ok(malformed)]);
    let process_source = FakeProcessTreeSource::new([(101, Vec::new()), (202, Vec::new())]);
    let mut service = DiscoveryService::with_sources(command, process_source);
    let initial = service.refresh().expect("first refresh succeeds");

    // When: tmux parsing fails before normalization can produce a partial snapshot.
    let failure = service.refresh().expect_err("second refresh fails");
    let display = failure.to_string();
    let debug = format!("{failure:?}");

    // Then: the typed error is sanitized and the last successful snapshot remains current.
    assert!(matches!(failure, TmuxError::MalformedField { .. }));
    assert!(!display.contains(secret));
    assert!(!debug.contains(secret));
    assert!(!display.contains("not-a-pid"));
    assert!(!debug.contains("not-a-pid"));
    assert_eq!(service.snapshot(), &initial);
}

#[test]
fn given_privacy_sensitive_tmux_command_failure_when_refreshed_then_previous_snapshot_is_untouched()
{
    // Given: one successful snapshot followed by a secret-bearing command failure.
    let first = format!(
        "{}\n",
        tmux_row(["w", "0", "a", "%1", "101", "0", "/t/a", "bash"])
    );
    let command = FakeTmuxCommand::new([
        Ok(first),
        Err(TmuxError::command_failed(
            Some(1),
            "/home/alice/private-repo token=super-secret",
        )),
    ]);
    let process_source = FakeProcessTreeSource::new([(101, Vec::new()), (202, Vec::new())]);
    let mut service = DiscoveryService::with_sources(command, process_source);
    let initial = service.refresh().expect("first refresh succeeds");

    // When: tmux execution fails before pane parsing.
    let failure = service.refresh().expect_err("second refresh fails");
    let display = failure.to_string();
    let debug = format!("{failure:?}");

    // Then: stderr is discarded and the last successful snapshot remains current.
    assert!(matches!(failure, TmuxError::CommandFailed { .. }));
    assert!(!display.contains("/home/alice/private-repo"));
    assert!(!debug.contains("/home/alice/private-repo"));
    assert!(!display.contains("super-secret"));
    assert!(!debug.contains("super-secret"));
    assert_eq!(service.snapshot(), &initial);
}

#[test]
fn given_system_tmux_command_when_constructing_discovery_service_then_production_constructor_compiles()
 {
    // Given/When: production discovery construction keeps the existing ergonomic tmux-only call.
    let service = DiscoveryService::new(SystemTmuxCommand::default());

    // Then: the constructor type-checks without requiring callers to name the process source.
    assert!(service.snapshot().is_empty());
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
