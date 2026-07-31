use agentmux::model::{AgentState, EvidenceConfidence};
use agentmux::process::classify_process;
use agentmux::tmux::{FieldErrorReason, TmuxError, parse_list_panes};

const FIELD_SEPARATOR: char = '\u{1f}';

fn tmux_row(fields: [&str; 8]) -> String {
    fields.join(&FIELD_SEPARATOR.to_string())
}

#[test]
fn given_tmux_rows_when_parsed_then_valid_rows_become_panes() {
    // Given: delimiter-separated tmux list-panes rows in the exact eight-field order.
    let output = format!(
        "{}\n{}\n",
        tmux_row([
            "work",
            "2",
            "api",
            "%1",
            "123",
            "0",
            "/home/alice/private-repo",
            "bash",
        ]),
        tmux_row(["ops", "10", "logs", "%2", "456", "1", "/", "zsh"]),
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
    let first_observation = first.observation(AgentState::Idle);
    assert_eq!(first_observation.session_name().as_str(), "work");
    assert_eq!(first_observation.window_index().get(), 2);
    assert_eq!(first_observation.window_name().as_str(), "api");
    assert_eq!(first_observation.workspace().as_str(), "private-repo");
    assert!(
        !first_observation
            .workspace()
            .as_str()
            .contains("/home/alice")
    );
    assert_eq!(second.process().pid(), Some(456));
    let second_observation = second.observation(AgentState::Exited);
    assert_eq!(second_observation.session_name().as_str(), "ops");
    assert_eq!(second_observation.window_index().get(), 10);
    assert_eq!(second_observation.window_name().as_str(), "logs");
    assert_eq!(second_observation.workspace().as_str(), "unknown");
    assert_eq!(
        second.process().evidence().confidence(),
        EvidenceConfidence::Medium
    );
    assert_eq!(classify_process(second.process()), AgentState::Exited);
}

#[test]
fn given_field_separator_in_any_tmux_field_when_parsed_then_row_is_rejected() {
    // Given: every tmux field position can contain a separator-bearing malicious value.
    let clean = [
        "work",
        "2",
        "api",
        "%1",
        "123",
        "0",
        "/home/alice/private-repo",
        "bash",
    ];

    for separator_position in 0..clean.len() {
        let embedded = format!("bad{FIELD_SEPARATOR}value");
        let fields = clean
            .iter()
            .enumerate()
            .map(|(field_position, value)| {
                if field_position == separator_position {
                    embedded.as_str()
                } else {
                    *value
                }
            })
            .collect::<Vec<_>>();
        let output = format!("{}\n", fields.join(&FIELD_SEPARATOR.to_string()));

        // When: the list-panes output is parsed.
        let result = parse_list_panes(&output);

        // Then: exact field counting rejects the row rather than preserving the separator.
        assert!(matches!(
            result,
            Err(TmuxError::MalformedFieldCount {
                row_index: 0,
                fields: 9
            })
        ));
    }
}

#[test]
fn given_invalid_tmux_rows_when_parsed_then_errors_are_typed() {
    // Given: malformed field count, window index, pid, boolean, pane id, and empty tmux outputs.
    let missing_fields =
        ["work", "2", "api", "%1", "123", "0", "/tmp"].join(&FIELD_SEPARATOR.to_string());
    let missing_fields = format!("{missing_fields}\n");
    let malformed_index = format!(
        "{}\n",
        tmux_row(["work", "abc", "api", "%1", "123", "0", "/tmp", "bash"])
    );
    let malformed_pid = format!(
        "{}\n",
        tmux_row(["work", "2", "api", "%1", "abc", "0", "/tmp", "bash"])
    );
    let malformed_bool = format!(
        "{}\n",
        tmux_row(["work", "2", "api", "%1", "123", "maybe", "/tmp", "bash"])
    );
    let invalid_pane_id = format!(
        "{}\n",
        tmux_row(["work", "2", "api", "not-a-pane", "123", "0", "/tmp", "bash"])
    );

    // When: each output is parsed.
    // Then: malformed data is rejected while empty output is a valid empty snapshot.
    assert!(matches!(
        parse_list_panes(&missing_fields),
        Err(TmuxError::MalformedFieldCount {
            row_index: 0,
            fields: 7
        })
    ));
    assert!(matches!(
        parse_list_panes(&malformed_index),
        Err(TmuxError::MalformedField {
            row_index: 0,
            field_name: "window_index",
            reason: FieldErrorReason::InvalidUnsignedInteger,
        })
    ));
    assert!(matches!(
        parse_list_panes(&malformed_pid),
        Err(TmuxError::MalformedField {
            row_index: 0,
            field_name: "pane_pid",
            reason: FieldErrorReason::InvalidUnsignedInteger,
        })
    ));
    assert!(matches!(
        parse_list_panes(&malformed_bool),
        Err(TmuxError::MalformedField {
            row_index: 0,
            field_name: "pane_dead",
            reason: FieldErrorReason::InvalidBoolean,
        })
    ));
    assert!(matches!(
        parse_list_panes(&invalid_pane_id),
        Err(TmuxError::MalformedField {
            row_index: 0,
            field_name: "pane_id",
            reason: FieldErrorReason::InvalidPaneId,
        })
    ));
    assert_eq!(
        parse_list_panes("").expect("empty output parses"),
        Vec::new()
    );
}
