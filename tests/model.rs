//! Public model and process classification tests.

use agentmux::model::{
    AgentId, AgentState, EvidenceConfidence, ModelError, PaneId, ProcessEvidence,
};
use agentmux::process::{classify_process, normalize_command_name};

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
    // Then: normal tmux-shaped IDs remain valid and control characters are rejected.
    assert_eq!(
        AgentId::new("pane:%1").expect("agent id is valid").as_str(),
        "pane:%1"
    );
    assert_eq!(PaneId::new("%1").expect("pane id is valid").as_str(), "%1");
    for invalid_id in invalid_ids {
        assert_eq!(AgentId::new(invalid_id), Err(ModelError::ControlCharacter));
        assert_eq!(PaneId::new(invalid_id), Err(ModelError::ControlCharacter));
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
