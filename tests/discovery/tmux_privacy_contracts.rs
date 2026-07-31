use agentmux::tmux::{FieldErrorReason, TmuxError, parse_list_panes};

const FIELD_SEPARATOR: char = '\u{1f}';

fn tmux_row(fields: [&str; 8]) -> String {
    fields.join(&FIELD_SEPARATOR.to_string())
}

#[test]
fn given_malformed_tmux_row_when_displayed_or_debugged_then_raw_data_is_not_retained() {
    // Given: a malformed row carries private path and token-like values.
    let private_path = "/home/alice/private-repo";
    let token = "TOKEN=super-secret";
    let output = format!(
        "{}\n",
        tmux_row([
            "work",
            "2",
            "api",
            "%1",
            "not-a-pid",
            "0",
            private_path,
            token,
        ])
    );

    // When: the row is parsed and formatted as an error.
    let error = parse_list_panes(&output).err();
    assert!(error.is_some(), "malformed row should be rejected");
    let Some(error) = error else {
        return;
    };
    let debug = format!("{error:?}");
    let display = error.to_string();

    // Then: typed metadata remains while raw row, raw value, path, and token are absent.
    assert!(matches!(
        error,
        TmuxError::MalformedField {
            row_index: 0,
            field_name: "pane_pid",
            reason: FieldErrorReason::InvalidUnsignedInteger,
        }
    ));
    for formatted in [debug.as_str(), display.as_str()] {
        assert!(!formatted.contains("not-a-pid"));
        assert!(!formatted.contains(private_path));
        assert!(!formatted.contains(token));
        assert!(!formatted.contains("work"));
    }
}

#[test]
fn given_control_character_in_tmux_field_when_parsed_then_error_is_sanitized() {
    // Given: a row containing private path data and token-like text plus invalid control chars.
    let secret_path = "/home/alice/private-repo";
    let token = "TOKEN=super-secret";
    let secrets = PrivacyProbe {
        path: secret_path,
        token,
    };
    let cases = control_character_cases(secret_path, token);

    for case in cases {
        assert_control_character_error(case, secrets);
    }
}

const fn control_character_cases<'a>(
    path: &'a str,
    token: &'a str,
) -> [MalformedControlCase<'a>; 8] {
    [
        MalformedControlCase::new(
            "session_name",
            ["bad\tsession", "2", "api", "%1", "123", "0", path, token],
            ["bad\tsession", path, token],
        ),
        MalformedControlCase::new(
            "window_index",
            ["work", "2\t", "api", "%1", "123", "0", path, token],
            ["2\t", path, token],
        ),
        MalformedControlCase::new(
            "window_name",
            ["work", "2", "bad\rwindow", "%1", "123", "0", path, token],
            ["bad\rwindow", path, token],
        ),
        MalformedControlCase::new(
            "pane_id",
            ["work", "2", "api", "%1\t", "123", "0", path, token],
            ["%1\t", path, token],
        ),
        MalformedControlCase::new(
            "pane_pid",
            ["work", "2", "api", "%1", "123\r", "0", path, token],
            ["123\r", path, token],
        ),
        MalformedControlCase::new(
            "pane_dead",
            ["work", "2", "api", "%1", "123", "0\t", path, token],
            ["0\t", path, token],
        ),
        MalformedControlCase::new(
            "pane_current_path",
            [
                "work",
                "2",
                "api",
                "%1",
                "123",
                "0",
                "/tmp/bad\tpath",
                token,
            ],
            ["/tmp/bad\tpath", path, token],
        ),
        MalformedControlCase::new(
            "pane_current_command",
            ["work", "2", "api", "%1", "123", "0", path, "bad\tcmd"],
            ["bad\tcmd", path, token],
        ),
    ]
}

#[test]
fn given_unicode_format_control_in_tmux_field_when_parsed_then_error_is_sanitized() {
    // Given: tmux display fields contain representative invisible format and bidi controls.
    let controls = [
        "\u{200b}", "\u{202e}", "\u{00ad}", "\u{034f}", "\u{061c}", "\u{070f}", "\u{180e}",
    ];

    for control in controls {
        let forbidden = format!("bad{control}session");
        let case = MalformedControlCase::new(
            "session_name",
            [
                &forbidden,
                "2",
                "api",
                "%1",
                "123",
                "0",
                "/home/alice/private-repo",
                "TOKEN=super-secret",
            ],
            [&forbidden, "/home/alice/private-repo", "TOKEN=super-secret"],
        );

        assert_control_character_error(
            case,
            PrivacyProbe {
                path: "/home/alice/private-repo",
                token: "TOKEN=super-secret",
            },
        );
    }
}

#[derive(Clone, Copy)]
struct MalformedControlCase<'a> {
    field_name: &'a str,
    reason: FieldErrorReason,
    fields: [&'a str; 8],
    forbidden_fragments: [&'a str; 3],
}

impl<'a> MalformedControlCase<'a> {
    const fn new(
        field_name: &'a str,
        fields: [&'a str; 8],
        forbidden_fragments: [&'a str; 3],
    ) -> Self {
        Self {
            field_name,
            reason: FieldErrorReason::ControlCharacter,
            fields,
            forbidden_fragments,
        }
    }
}

#[derive(Clone, Copy)]
struct PrivacyProbe<'a> {
    path: &'a str,
    token: &'a str,
}

fn assert_control_character_error(case: MalformedControlCase<'_>, secrets: PrivacyProbe<'_>) {
    // When: the list-panes output is parsed.
    let row = tmux_row(case.fields);
    let error = parse_list_panes(&format!("{row}\n")).err();
    assert!(error.is_some(), "control character row should be rejected");
    let Some(error) = error else {
        return;
    };
    let debug = format!("{error:?}");
    let display = error.to_string();

    // Then: typed metadata identifies the field while raw row, path, token, tabs, and newlines do not leak.
    assert!(matches!(
        error,
        TmuxError::MalformedField {
            row_index: 0,
            field_name: actual,
            reason: actual_reason,
        } if actual == case.field_name && actual_reason == case.reason
    ));
    for fragment in case.forbidden_fragments {
        assert!(!debug.contains(fragment));
        assert!(!display.contains(fragment));
        assert!(!debug.contains(&format!("prefix{fragment}")));
        assert!(!display.contains(&format!("prefix{fragment}")));
        assert!(!debug.contains(&format!("{fragment}suffix")));
        assert!(!display.contains(&format!("{fragment}suffix")));
        assert_escaped_fragment_is_absent(&debug, fragment);
        assert_escaped_fragment_is_absent(&display, fragment);
    }
    assert!(!debug.contains(secrets.path));
    assert!(!debug.contains(secrets.token));
    assert!(!debug.contains('\t'));
    assert!(!debug.contains('\n'));
    assert!(!display.contains(secrets.path));
    assert!(!display.contains(secrets.token));
    assert!(!display.contains('\t'));
    assert!(!display.contains('\n'));
}

fn assert_escaped_fragment_is_absent(formatted: &str, fragment: &str) {
    let escaped = fragment.escape_debug().to_string();
    if escaped != fragment {
        assert!(!formatted.contains(&escaped));
    }

    let printable: String = fragment
        .chars()
        .filter(|character| !is_disallowed_display_control(*character))
        .collect();
    if printable != fragment && printable.len() >= 4 {
        assert!(!formatted.contains(&printable));
    }
}

fn is_disallowed_display_control(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{00ad}' | '\u{034f}' | '\u{061c}' | '\u{070f}' | '\u{180e}' | '\u{200b}'
                ..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

#[test]
fn given_secret_bearing_command_failure_when_debugged_then_stderr_is_not_retained() {
    // Given: a command failure created from secret-bearing tmux stderr.
    let error = TmuxError::command_failed(Some(1), "/home/alice/private-repo token=super-secret");

    // When: callers inspect Display and Debug.
    let debug = format!("{error:?}");
    let display = error.to_string();

    // Then: only status and fixed context remain.
    assert!(matches!(
        error,
        TmuxError::CommandFailed { status: Some(1) }
    ));
    assert!(!debug.contains("private-repo"));
    assert!(!debug.contains("super-secret"));
    assert!(!display.contains("private-repo"));
    assert!(!display.contains("super-secret"));
}
