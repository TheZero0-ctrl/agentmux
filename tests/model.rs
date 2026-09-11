//! Public model and process classification tests.

use agentmux::model::{
    AgentId, AgentState, Evidence, EvidenceConfidence, EvidenceFreshness, EvidenceSource,
    ModelError, Pane, PaneId, ProcessEvidence, ProcessLiveness,
};
use agentmux::process::{classify_process, normalize_command_name};

#[path = "model/discovery_contracts.rs"]
mod discovery_contracts;

#[test]
fn given_branded_ids_and_states_when_used_then_agent_state_is_typed() {
    // Given: branded agent and pane identifiers plus every approved state.
    let agent_id = AgentId::new("agent:%1").expect("agent id is valid");
    let pane_id = PaneId::new("%1").expect("pane id is valid");
    let states = [
        AgentState::Idle,
        AgentState::Working,
        AgentState::WaitingPermission,
        AgentState::WaitingPlanApproval,
        AgentState::WaitingQuestion,
        AgentState::Unknown,
        AgentState::Exited,
    ];

    // When: the identifiers and states are observed through the public model.
    // Then: the semantic identifiers remain distinct and all states are representable.
    assert_eq!(agent_id.as_str(), "agent:%1");
    assert_eq!(pane_id.as_str(), "%1");
    assert_eq!(states.len(), 7);
}

#[test]
fn given_control_characters_when_ids_are_created_then_they_are_rejected() {
    // Given: normal IDs plus TSV-breaking and control-character identifiers.
    let invalid_ids = [
        "\t",
        "\n",
        "\r",
        "\u{1f}",
        "agent\t1",
        "agent\n1",
        "agent\r1",
        "agent\u{1f}1",
    ];

    // When: IDs cross the public model boundary.
    // Then: normal tmux-shaped pane IDs remain valid and control characters are rejected.
    assert_eq!(PaneId::new("%1").expect("pane id is valid").as_str(), "%1");
    for invalid_id in invalid_ids {
        assert_eq!(AgentId::new(invalid_id), Err(ModelError::ControlCharacter));
        assert_eq!(PaneId::new(invalid_id), Err(ModelError::ControlCharacter));
    }
}

#[test]
fn given_pane_derived_agent_namespace_when_created_directly_then_it_is_reserved() {
    // Given: raw identifiers that use the pane-derived agent namespace.
    let pane_id = PaneId::new("%1").expect("pane id is valid");
    let reserved_agent_ids = ["pane:%1", "pane:custom", "pane:%abc"];

    // When: callers use the public AgentId constructor instead of the pane constructor.
    let derived = AgentId::from_pane_id(&pane_id);

    // Then: the namespace is reserved while pane-derived IDs remain deterministic.
    for reserved_agent_id in reserved_agent_ids {
        assert_eq!(
            AgentId::new(reserved_agent_id),
            Err(ModelError::InvalidPaneIdFormat)
        );
    }
    assert_eq!(derived.as_str(), "pane:%1");
}

#[test]
fn given_display_controls_when_ids_are_created_then_they_are_rejected() {
    // Given: invisible and bidirectional controls that can alter displayed identifiers.
    let invalid_ids = [
        "agent\u{202e}1",
        "agent\u{200b}1",
        "%1\u{202e}",
        "%1\u{200b}",
    ];

    // When: IDs cross the public model boundary.
    // Then: displayed IDs reject them while normal tmux pane syntax remains valid.
    assert_eq!(
        PaneId::new("%12").expect("pane id is valid").as_str(),
        "%12"
    );
    for invalid_id in invalid_ids {
        assert_eq!(AgentId::new(invalid_id), Err(ModelError::ControlCharacter));
        assert_eq!(PaneId::new(invalid_id), Err(ModelError::ControlCharacter));
    }
}

#[test]
fn given_empty_ids_when_created_then_they_are_rejected() {
    // Given: empty and whitespace-only identifiers at the public model boundary.
    let empty_ids = ["", " ", "   "];

    // When: agent and pane IDs are created.
    // Then: both branded ID types reject values that are empty after trimming.
    for empty_id in empty_ids {
        assert_eq!(AgentId::new(empty_id), Err(ModelError::EmptyId));
        assert_eq!(PaneId::new(empty_id), Err(ModelError::EmptyId));
    }
}

#[test]
fn given_pane_ids_when_created_then_only_tmux_percent_ascii_digits_are_valid() {
    // Given: valid tmux pane IDs and malformed non-control pane IDs.
    let valid_pane_ids = ["%1", "%123456"];
    let invalid_pane_ids = [
        "1", "pane:%1", "%", "%abc", "%1a", "%%1", "%-1", "% 1", "%１２",
    ];

    // When: pane IDs cross the public model boundary.
    // Then: only percent-prefixed ASCII digit pane IDs are accepted.
    for valid_pane_id in valid_pane_ids {
        assert_eq!(
            PaneId::new(valid_pane_id)
                .expect("pane id is valid")
                .as_str(),
            valid_pane_id
        );
    }
    for invalid_pane_id in invalid_pane_ids {
        assert_eq!(
            PaneId::new(invalid_pane_id),
            Err(ModelError::InvalidPaneIdFormat)
        );
    }
    assert_eq!(PaneId::new(""), Err(ModelError::EmptyId));
    assert_eq!(PaneId::new("\t"), Err(ModelError::ControlCharacter));
}

#[test]
fn given_process_evidence_when_classified_then_only_generic_shell_rules_are_inferred() {
    // Given: live shell, live command, dead, incomplete, and stale process evidence.
    let shell_commands = [
        "sh",
        "/usr/bin/-bash",
        "zsh",
        "fish",
        "nu",
        "dash",
        "ksh",
        "mksh",
        "csh",
        "tcsh",
        "elvish",
        "xonsh",
        "pwsh",
        "powershell",
    ];
    let working_commands = ["python", "nvim"];
    let dead = ProcessEvidence::dead(125, "zsh");
    let incomplete = ProcessEvidence::incomplete_live(126);
    let stale = ProcessEvidence::live(127, "python").into_stale();

    // When: process evidence is classified.
    // Then: only generic shell/non-shell/dead/incomplete/stale states are inferred.
    for command in shell_commands {
        assert_eq!(
            classify_process(&ProcessEvidence::live(123, command)),
            AgentState::Idle
        );
    }
    for command in working_commands {
        assert_eq!(
            classify_process(&ProcessEvidence::live(124, command)),
            AgentState::Working
        );
    }
    assert_eq!(classify_process(&dead), AgentState::Exited);
    assert_eq!(dead.evidence().confidence(), EvidenceConfidence::Medium);
    assert_eq!(classify_process(&incomplete), AgentState::Unknown);
    assert_eq!(classify_process(&stale), AgentState::Unknown);
    assert_eq!(normalize_command_name("/usr/bin/-bash"), Some("bash"));
    assert_eq!(normalize_command_name("/usr/bin/fish"), Some("fish"));
    assert_eq!(normalize_command_name("-zsh"), Some("zsh"));
}

#[test]
fn given_evidence_when_observed_then_accessors_return_current_contract_values() {
    // Given: direct evidence metadata plus process evidence from each constructor.
    let evidence = Evidence::new(
        EvidenceSource::Process,
        EvidenceFreshness::Fresh,
        EvidenceConfidence::Low,
    );
    let live = ProcessEvidence::live(201, "agentmux");
    let dead = ProcessEvidence::dead(202, "agentmux");
    let incomplete = ProcessEvidence::incomplete_live(203);
    let tmux = ProcessEvidence::from_tmux(204, "bash", ProcessLiveness::Unknown);

    // When: callers read values through public accessors.
    // Then: metadata, PID, command, liveness, and confidence remain unchanged.
    assert_eq!(evidence.source(), EvidenceSource::Process);
    assert_eq!(evidence.freshness(), EvidenceFreshness::Fresh);
    assert_eq!(evidence.confidence(), EvidenceConfidence::Low);
    assert_eq!(live.pid(), Some(201));
    assert_eq!(live.command(), Some("agentmux"));
    assert_eq!(live.liveness(), ProcessLiveness::Live);
    assert_eq!(live.evidence().source(), EvidenceSource::Process);
    assert_eq!(dead.liveness(), ProcessLiveness::Dead);
    assert_eq!(dead.evidence().confidence(), EvidenceConfidence::Medium);
    assert_eq!(incomplete.pid(), Some(203));
    assert_eq!(incomplete.command(), None);
    assert_eq!(incomplete.liveness(), ProcessLiveness::Live);
    assert_eq!(tmux.evidence().source(), EvidenceSource::Tmux);
    assert_eq!(tmux.liveness(), ProcessLiveness::Unknown);
}

#[test]
fn given_pane_observation_when_process_evidence_differs_then_status_uses_pane_process_evidence() {
    // Given: tmux-sourced pane process evidence and separately constructable process evidence.
    let pane = Pane::new(
        PaneId::new("%9").expect("pane id is valid"),
        ProcessEvidence::from_tmux(901, "bash", ProcessLiveness::Live),
    );
    let process_evidence = ProcessEvidence::live(901, "bash").evidence();

    // When: an observation is built from the pane.
    let observation = pane.observation(AgentState::Idle);

    // Then: callers cannot inject mismatched evidence through Pane::observation.
    assert_ne!(observation.evidence(), process_evidence);
    assert_eq!(observation.evidence().source(), EvidenceSource::Tmux);
}

#[test]
fn given_process_evidence_when_debugged_then_command_text_is_redacted() {
    // Given: process evidence with command text that may contain paths, argv, or secrets.
    let evidence = ProcessEvidence::live(777, "/home/alice/private --token=super-secret");

    // When: callers format Debug output.
    let debug = format!("{evidence:?}");

    // Then: PID/liveness metadata remains useful while raw command text is absent.
    assert!(debug.contains("pid"));
    assert!(debug.contains("Live"));
    assert!(!debug.contains("/home/alice/private"));
    assert!(!debug.contains("super-secret"));
}

#[test]
fn given_padded_process_command_when_observed_then_stored_command_is_trimmed() {
    // Given: process evidence created from a padded tmux command.
    let padded = ProcessEvidence::from_tmux(301, " bash ", ProcessLiveness::Live);
    let unpadded = ProcessEvidence::from_tmux(301, "bash", ProcessLiveness::Live);

    // When: callers observe the stored command and compare evidence values.
    // Then: the command is stored once in normalized form.
    assert_eq!(padded.command(), Some("bash"));
    assert_eq!(padded, unpadded);
}

#[test]
fn given_whitespace_commands_when_normalized_then_current_basename_rules_hold() {
    // Given: whitespace, paths, arguments, and login-shell command strings.
    let commands = [
        ("", None),
        ("   ", None),
        ("  /usr/bin/bash  ", Some("bash")),
        (" /usr/bin/-zsh ", Some("zsh")),
        (" /usr/bin/opencode --continue ", Some("opencode")),
        ("  python  ", Some("python")),
    ];

    // When: command strings are normalized.
    // Then: trimming, basename extraction, and login-shell stripping stay stable.
    for (command, expected) in commands {
        assert_eq!(normalize_command_name(command), expected);
    }
}
