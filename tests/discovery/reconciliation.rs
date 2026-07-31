use agentmux::model::{
    AgentState, ClientCandidate, ClientConfidence, ClientKind, EvidenceConfidence,
    EvidenceFreshness, EvidenceSource, LocationMetadata, ModelError, Pane, PaneId, ProcessBasename,
    ProcessEvidence, ProcessIdentity, ProcessMetadata, SessionName, SessionWindow, WindowIndex,
    WindowName, WorkspaceLabel,
};
use agentmux::state::{AgentSnapshot, normalize_snapshot};

#[test]
fn given_same_pane_and_same_identity_when_normalized_then_current_observation_replaces_previous()
-> Result<(), ModelError> {
    // Given: one pane-derived agent first observed with safe location and process metadata.
    let identity = ProcessIdentity::new(101, 1_001);
    let first = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::live(101, "bash"),
            known_process(identity, "opencode", ClientKind::OpenCode)?,
        )?],
    );

    // When: the same pane and same process identity refresh with changed safe metadata.
    let second = normalize_snapshot(
        &first,
        vec![enriched_pane(
            "%1",
            "worker",
            "private-worker",
            ProcessEvidence::live(101, "python"),
            known_process(identity, "codex", ClientKind::Codex)?,
        )?],
    );
    let refreshed = second.agent("pane:%1").expect("same pane remains tracked");

    // Then: the pane-derived key remains stable and current observation data wins.
    assert_eq!(second.len(), 1);
    assert_eq!(refreshed.id().as_str(), "pane:%1");
    assert_eq!(refreshed.state(), AgentState::Working);
    assert_eq!(refreshed.observation().window_name().as_str(), "worker");
    assert_eq!(
        refreshed.observation().workspace().as_str(),
        "private-worker"
    );
    assert_eq!(refreshed.observation().process_identity(), Some(&identity));
    assert_eq!(
        refreshed.observation().client_candidate(),
        ClientCandidate::known(ClientKind::Codex, ClientConfidence::Low)
    );
    Ok(())
}

#[test]
fn given_same_pane_and_reused_pid_when_normalized_then_prior_candidate_certainty_is_not_carried()
-> Result<(), ModelError> {
    // Given: a known candidate for a pane PID before Linux start-time changes.
    let first = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::live(101, "bash"),
            known_process(
                ProcessIdentity::new(101, 1_001),
                "opencode",
                ClientKind::OpenCode,
            )?,
        )?],
    );

    // When: the same pane and PID refresh with a different start-time and no candidate proof.
    let second = normalize_snapshot(
        &first,
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::live(101, "bash"),
            ProcessMetadata::new(
                Some(ProcessIdentity::new(101, 2_002)),
                None,
                ClientCandidate::Unknown,
            ),
        )?],
    );
    let reused = second.agent("pane:%1").expect("reused pid pane is present");

    // Then: PID reuse keeps the new identity and does not inherit the old candidate.
    assert_eq!(
        reused.observation().process_identity(),
        Some(&ProcessIdentity::new(101, 2_002))
    );
    assert_eq!(
        reused.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    assert_eq!(reused.observation().process_basename(), None);
    Ok(())
}

#[test]
fn given_unknown_identity_when_normalized_then_candidate_remains_unknown() -> Result<(), ModelError>
{
    // Given: a pane whose process tree did not prove a stable process identity.
    let snapshot = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::live(101, "bash"),
            ProcessMetadata::unknown(),
        )?],
    );

    // When: callers inspect the normalized agent.
    let unknown = snapshot.agent("pane:%1").expect("pane is present");

    // Then: unknown identity cannot become candidate certainty.
    assert_eq!(unknown.observation().process_identity(), None);
    assert_eq!(
        unknown.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    Ok(())
}

#[test]
fn given_exited_present_pane_when_missing_once_then_safe_location_metadata_is_preserved()
-> Result<(), ModelError> {
    // Given: a pane that was present with a dead process and enriched safe metadata.
    let first = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::dead(101, "opencode"),
            known_process(
                ProcessIdentity::new(101, 1_001),
                "opencode",
                ClientKind::OpenCode,
            )?,
        )?],
    );

    // When: the next successful snapshot no longer contains that pane.
    let second = normalize_snapshot(&first, Vec::new());
    let missing = second
        .agent("pane:%1")
        .expect("first missing refresh tombstones");

    // Then: one exited tombstone remains with missing-pane evidence and safe metadata only.
    assert_eq!(second.len(), 1);
    assert_eq!(missing.state(), AgentState::Exited);
    assert_eq!(missing.evidence().source(), EvidenceSource::MissingPane);
    assert_eq!(missing.evidence().freshness(), EvidenceFreshness::Fresh);
    assert_eq!(missing.evidence().confidence(), EvidenceConfidence::Medium);
    assert_eq!(missing.observation().session_name().as_str(), "work");
    assert_eq!(missing.observation().window_name().as_str(), "api");
    assert_eq!(missing.observation().workspace().as_str(), "private-api");
    assert_eq!(missing.observation().process_identity(), None);
    assert_eq!(missing.observation().process_basename(), None);
    assert_eq!(
        missing.observation().client_candidate(),
        ClientCandidate::Unknown
    );
    Ok(())
}

#[test]
fn given_missing_pane_tombstone_when_missing_again_then_it_is_pruned() -> Result<(), ModelError> {
    // Given: a present pane followed by one successful missing-pane refresh.
    let first = normalize_snapshot(
        &AgentSnapshot::default(),
        vec![enriched_pane(
            "%1",
            "api",
            "private-api",
            ProcessEvidence::live(101, "bash"),
            ProcessMetadata::unknown(),
        )?],
    );
    let tombstone = normalize_snapshot(&first, Vec::new());

    // When: a second successful refresh still does not contain the pane.
    let pruned = normalize_snapshot(&tombstone, Vec::new());

    // Then: the one-refresh tombstone is removed.
    assert_eq!(tombstone.agent_state("pane:%1"), Some(AgentState::Exited));
    assert_eq!(pruned.agent_state("pane:%1"), None);
    assert_eq!(pruned.len(), 0);
    Ok(())
}

fn enriched_pane(
    pane_id: &str,
    window_name: &str,
    workspace: &str,
    process: ProcessEvidence,
    process_metadata: ProcessMetadata,
) -> Result<Pane, ModelError> {
    Ok(Pane::with_location(
        LocationMetadata::new(
            PaneId::new(pane_id)?,
            SessionWindow::new(
                SessionName::new("work")?,
                WindowIndex::new(0),
                WindowName::new(window_name)?,
            ),
            WorkspaceLabel::from_path(std::path::Path::new(workspace))?,
        ),
        process,
    )
    .with_process_metadata(process_metadata))
}

fn known_process(
    identity: ProcessIdentity,
    basename: &str,
    kind: ClientKind,
) -> Result<ProcessMetadata, ModelError> {
    Ok(ProcessMetadata::new(
        Some(identity),
        Some(ProcessBasename::new(basename)?),
        ClientCandidate::known(kind, ClientConfidence::Low),
    ))
}
