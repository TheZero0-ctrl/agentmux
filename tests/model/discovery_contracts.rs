use agentmux::model::{
    AgentObservation, AgentState, ClientCandidate, ClientConfidence, ClientKind, Evidence,
    EvidenceConfidence, EvidenceFreshness, EvidenceSource, LocationMetadata, ModelError,
    ObservationStatus, PaneId, ProcessBasename, ProcessIdentity, ProcessMetadata, SessionName,
    SessionWindow, WindowIndex, WindowName, WorkspaceLabel,
};
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

#[test]
fn given_discovery_identity_metadata_and_privacy_when_constructed_then_contracts_hold()
-> Result<(), ModelError> {
    // Given: process identity, tmux metadata, and safe labels from untrusted discovery input.
    let first = ProcessIdentity::new(42, 10_001);
    let reused = ProcessIdentity::new(42, 10_002);
    let workspace = WorkspaceLabel::from_path(Path::new("/home/alice/secret-repo"))?;
    let empty = WorkspaceLabel::from_path(Path::new(""))?;
    let root = WorkspaceLabel::from_path(Path::new("/"))?;
    #[cfg(unix)]
    let non_utf = WorkspaceLabel::from_path(Path::new(&OsString::from_vec(vec![0xff])))?;

    // When: callers observe the typed values through public accessors.
    // Then: PID reuse is distinct and only safe labels are stored/displayed.
    assert_ne!(first, reused);
    assert_eq!((first.pid(), first.start_time_ticks()), (42, 10_001));
    assert_eq!(
        (
            SessionName::new("work")?.as_str(),
            WindowIndex::new(7).get(),
            WindowName::new("editor")?.as_str()
        ),
        ("work", 7, "editor")
    );
    assert_eq!(
        (workspace.as_str(), workspace.to_string().as_str()),
        ("secret-repo", "secret-repo")
    );
    assert_eq!((empty.as_str(), root.as_str()), ("unknown", "unknown"));
    #[cfg(unix)]
    assert_eq!(non_utf.as_str(), "unknown");
    Ok(())
}

#[test]
fn given_windows_style_workspace_paths_when_constructed_then_only_final_component_is_kept()
-> Result<(), ModelError> {
    // Given: Windows and UNC style paths crossing the workspace label boundary on Unix.
    let windows = WorkspaceLabel::from_path(Path::new(r"C:\Users\alice\private-repo"))?;
    let unc = WorkspaceLabel::from_path(Path::new(r"\\server\share\customer-secret"))?;

    // When: callers observe the safe label text.
    let labels = [windows.as_str(), unc.as_str()];

    // Then: only the final component survives and parent/user/share names are absent.
    assert_eq!(labels, ["private-repo", "customer-secret"]);
    for label in labels {
        assert!(!label.contains("Users"));
        assert!(!label.contains("alice"));
        assert!(!label.contains("server"));
        assert!(!label.contains("share"));
    }
    Ok(())
}

#[test]
fn given_malformed_discovery_labels_when_constructed_then_boundaries_reject_them()
-> Result<(), ModelError> {
    // Given: control characters and path-bearing executable display attempts.
    for value in ["agent\t1", "agent\n1", "agent\r1", "agent\u{1f}1"] {
        assert_eq!(SessionName::new(value), Err(ModelError::ControlCharacter));
        assert_eq!(WindowName::new(value), Err(ModelError::ControlCharacter));
        assert_eq!(
            ProcessBasename::new(value),
            Err(ModelError::ControlCharacter)
        );
    }

    // When: executable names cross the basename boundary.
    // Then: path components, empty names, and controls cannot enter model state.
    assert_eq!(ProcessBasename::new("opencode")?.as_str(), "opencode");
    for value in ["/usr/bin/opencode", "bin/opencode"] {
        assert_eq!(ProcessBasename::new(value), Err(ModelError::PathComponent));
    }
    assert_eq!(ProcessBasename::new(""), Err(ModelError::EmptyId));
    Ok(())
}

#[test]
fn given_unicode_format_controls_when_constructed_then_display_labels_reject_them() {
    // Given: invisible Unicode controls that can alter or hide display text.
    let controls = [
        '\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}',
        '\u{202c}', '\u{202d}', '\u{202e}', '\u{2060}', '\u{2061}', '\u{2062}', '\u{2063}',
        '\u{2064}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{feff}', '\u{00ad}',
        '\u{034f}', '\u{061c}', '\u{070f}', '\u{180e}',
    ];

    for control in controls {
        let value = format!("agent{control}label");

        // When: the value crosses any display-label boundary.
        // Then: the boundary rejects the invisible control before storing it.
        assert_eq!(SessionName::new(&value), Err(ModelError::ControlCharacter));
        assert_eq!(WindowName::new(&value), Err(ModelError::ControlCharacter));
        assert_eq!(
            WorkspaceLabel::from_path(Path::new(&value)),
            Err(ModelError::ControlCharacter)
        );
        assert_eq!(
            ProcessBasename::new(&value),
            Err(ModelError::ControlCharacter)
        );
    }
}

#[test]
fn given_sensitive_labels_when_debugged_then_location_debug_is_redacted() -> Result<(), ModelError>
{
    // Given: typed labels that may contain sensitive tmux names after boundary parsing.
    let session = SessionName::new("secret-session-token")?;
    let window = WindowName::new("diff --git private.patch")?;
    let workspace = WorkspaceLabel::from_path(Path::new("/home/alice/customer-secret"))?;

    // When: wrappers and a containing observation are formatted with Debug.
    let session_debug = format!("{session:?}");
    let window_debug = format!("{window:?}");
    let workspace_debug = format!("{workspace:?}");
    let observation = AgentObservation::new(
        LocationMetadata::new(
            PaneId::new("%9")?,
            SessionWindow::new(session, WindowIndex::new(4), window),
            workspace,
        ),
        ProcessMetadata::unknown(),
        ObservationStatus::new(
            AgentState::Unknown,
            Evidence::new(
                EvidenceSource::Tmux,
                EvidenceFreshness::Fresh,
                EvidenceConfidence::Low,
            ),
        ),
    );
    let observation_debug = format!("{observation:?}");

    // Then: debug output keeps type names useful without exposing raw labels.
    for formatted in [
        session_debug,
        window_debug,
        workspace_debug,
        observation_debug,
    ] {
        assert!(!formatted.contains("secret-session-token"));
        assert!(!formatted.contains("diff --git"));
        assert!(!formatted.contains("customer-secret"));
        assert!(formatted.contains("redacted"));
    }
    Ok(())
}

#[test]
fn given_enriched_observation_when_read_then_no_full_paths_or_argv_are_exposed()
-> Result<(), ModelError> {
    // Given: an enriched observation assembled only from privacy-safe values.
    let identity = ProcessIdentity::new(9042, 88_100);
    let basename = ProcessBasename::new("opencode")?;
    let candidate = ClientCandidate::known(ClientKind::OpenCode, ClientConfidence::Low);
    let observation = AgentObservation::new(
        LocationMetadata::new(
            PaneId::new("%3")?,
            SessionWindow::new(
                SessionName::new("work")?,
                WindowIndex::new(2),
                WindowName::new("editor")?,
            ),
            WorkspaceLabel::from_path(Path::new("/home/alice/secret-repo"))?,
        ),
        ProcessMetadata::new(Some(identity), Some(basename.clone()), candidate),
        ObservationStatus::new(
            AgentState::Working,
            Evidence::new(
                EvidenceSource::Process,
                EvidenceFreshness::Fresh,
                EvidenceConfidence::Low,
            ),
        ),
    );

    // When: callers read the shared observation fields.
    // Then: AgentId is pane-derived, candidate confidence is explicit, and secrets stay absent.
    assert_eq!(
        (
            observation.agent_id().as_str(),
            observation.pane_id().as_str()
        ),
        ("pane:%3", "%3")
    );
    assert_eq!(
        (
            observation.session_name().as_str(),
            observation.window_index().get(),
            observation.window_name().as_str()
        ),
        ("work", 2, "editor")
    );
    assert_eq!(
        (
            observation.process_identity(),
            observation.process_basename()
        ),
        (Some(&identity), Some(&basename))
    );
    assert_eq!(
        (
            candidate.kind(),
            candidate.confidence(),
            observation.client_candidate().label()
        ),
        (
            Some(ClientKind::OpenCode),
            Some(ClientConfidence::Low),
            "opencode"
        )
    );
    assert_eq!(
        (
            ClientCandidate::unknown().kind(),
            ClientCandidate::unknown().confidence(),
            ClientCandidate::unknown().label()
        ),
        (None, None, "unknown")
    );
    assert_eq!(
        (
            observation.workspace().as_str(),
            observation.state(),
            observation.evidence().source()
        ),
        ("secret-repo", AgentState::Working, EvidenceSource::Process)
    );
    assert!(!observation.workspace().as_str().contains("/home/alice"));
    assert!(!basename.as_str().contains('/'));
    assert!(!basename.as_str().contains("--danger-token"));
    Ok(())
}
