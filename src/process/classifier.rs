use crate::model::{
    AgentState, ClientCandidate, ClientConfidence, ClientKind, EvidenceFreshness, ProcessBasename,
    ProcessEvidence, ProcessLiveness, ProcessMetadata,
};

use super::command::{is_shell_command, normalize_command_name};
use super::procfs::ProcessRecord;

/// Classify generic process evidence into a conservative agent state.
#[must_use]
pub fn classify_process(process: &ProcessEvidence) -> AgentState {
    match process.evidence().freshness() {
        EvidenceFreshness::Stale => AgentState::Unknown,
        EvidenceFreshness::Fresh => match process.liveness() {
            ProcessLiveness::Live => match process.command().and_then(normalize_command_name) {
                Some(command) if is_shell_command(command) => AgentState::Idle,
                Some(_) => AgentState::Working,
                None => AgentState::Unknown,
            },
            ProcessLiveness::Dead => AgentState::Exited,
            ProcessLiveness::Unknown => AgentState::Unknown,
        },
    }
}

/// Classify a procfs process tree into conservative selected process metadata.
#[must_use]
pub fn classify_process_tree(records: &[ProcessRecord]) -> ProcessMetadata {
    let Some(selected) = selected_candidate(records) else {
        return ProcessMetadata::unknown();
    };
    ProcessMetadata::new(
        Some(selected.record.identity()),
        selected.record.basename().cloned(),
        ClientCandidate::known(selected.kind, ClientConfidence::Low),
    )
}

struct SelectedCandidate<'a> {
    record: &'a ProcessRecord,
    kind: ClientKind,
}

fn selected_candidate(records: &[ProcessRecord]) -> Option<SelectedCandidate<'_>> {
    let mut selected: Option<SelectedCandidate<'_>> = None;
    for record in records {
        let Some(basename) = record.basename() else {
            continue;
        };
        if let Some(kind) = candidate_kind(basename) {
            match selected {
                Some(existing) if existing.kind != kind => return None,
                Some(_) => {}
                None => selected = Some(SelectedCandidate { record, kind }),
            }
        }
    }
    selected
}

fn candidate_kind(basename: &ProcessBasename) -> Option<ClientKind> {
    match basename.as_str() {
        "opencode" => Some(ClientKind::OpenCode),
        "codex" => Some(ClientKind::Codex),
        "claude" => Some(ClientKind::Claude),
        "gemini" => Some(ClientKind::Gemini),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        AgentState, ClientCandidate, ClientConfidence, ClientKind, ModelError, ProcessBasename,
        ProcessIdentity, ProcessMetadata,
    };

    use super::super::procfs::ProcessRecord;
    use super::{classify_process, classify_process_tree};
    use crate::model::ProcessEvidence;

    #[test]
    fn given_exact_supported_basenames_when_classified_then_low_confidence_candidates_are_returned()
    -> Result<(), ModelError> {
        // Given: process-tree records with exact executable basenames supported by this milestone.
        let cases = [
            (
                "opencode",
                ClientKind::OpenCode,
                ProcessIdentity::new(100, 1_000),
            ),
            ("codex", ClientKind::Codex, ProcessIdentity::new(101, 1_001)),
            (
                "claude",
                ClientKind::Claude,
                ProcessIdentity::new(102, 1_002),
            ),
            (
                "gemini",
                ClientKind::Gemini,
                ProcessIdentity::new(103, 1_003),
            ),
        ];

        // When: each tree is classified through the pure process-tree seam.
        // Then: the candidate is only a low-confidence hint and the selected record is preserved.
        for (basename, kind, identity) in cases {
            let metadata = classify_process_tree(&[record(identity, basename)?]);
            assert_known(&metadata, identity, basename, kind)?;
        }
        Ok(())
    }

    #[test]
    fn given_same_kind_duplicates_when_classified_then_nearest_stable_record_is_selected()
    -> Result<(), ModelError> {
        // Given: a root command plus two same-kind descendant candidates in process-tree order.
        let nearest = ProcessIdentity::new(20, 200);
        let later = ProcessIdentity::new(30, 300);
        let records = [
            record(ProcessIdentity::new(10, 100), "bash")?,
            record(nearest, "codex")?,
            record(later, "codex")?,
        ];

        // When: the tree is classified.
        let metadata = classify_process_tree(&records);

        // Then: the first stable record in root-to-descendant order wins; same-depth ties are PID-ordered by collection.
        assert_known(&metadata, nearest, "codex", ClientKind::Codex)?;
        Ok(())
    }

    #[test]
    fn given_unsupported_or_incomplete_trees_when_classified_then_candidate_is_unknown()
    -> Result<(), ModelError> {
        // Given: lookalikes, generic names, case variants, absent evidence, and conflicting kinds.
        let cases = [
            vec![record(ProcessIdentity::new(1, 10), "codex-helper")?],
            vec![record(ProcessIdentity::new(2, 20), "pi")?],
            vec![record(ProcessIdentity::new(3, 30), "amp")?],
            vec![record(ProcessIdentity::new(4, 40), "OpenCode")?],
            vec![record_without_basename(ProcessIdentity::new(5, 50))],
            vec![
                record(ProcessIdentity::new(6, 60), "opencode")?,
                record(ProcessIdentity::new(7, 70), "codex")?,
            ],
            Vec::new(),
        ];

        // When: each tree is classified.
        // Then: anything missing, degraded, generic, similar, or conflicting remains unknown.
        for records in cases {
            assert_eq!(classify_process_tree(&records), ProcessMetadata::unknown());
        }
        Ok(())
    }

    #[test]
    fn given_incomplete_record_before_supported_basename_when_classified_then_supported_candidate_wins()
    -> Result<(), ModelError> {
        // Given: procfs could read the shell identity but not its executable basename before a supported child.
        let identity = ProcessIdentity::new(20, 200);
        let records = [
            record_without_basename(ProcessIdentity::new(10, 100)),
            record(identity, "opencode")?,
        ];

        // When: the tree is classified.
        let metadata = classify_process_tree(&records);

        // Then: incomplete records are skipped instead of aborting the scan.
        assert_known(&metadata, identity, "opencode", ClientKind::OpenCode)?;
        Ok(())
    }

    #[test]
    fn given_incomplete_record_between_conflicting_candidates_when_classified_then_unknown_remains()
    -> Result<(), ModelError> {
        // Given: incomplete records do not hide conflicts between supported candidate kinds.
        let records = [
            record(ProcessIdentity::new(10, 100), "opencode")?,
            record_without_basename(ProcessIdentity::new(15, 150)),
            record(ProcessIdentity::new(20, 200), "codex")?,
        ];

        // When: the tree is classified.
        let metadata = classify_process_tree(&records);

        // Then: conflicting supported candidates are still conservative unknowns.
        assert_eq!(metadata, ProcessMetadata::unknown());
        Ok(())
    }

    #[test]
    fn given_supported_basename_process_evidence_when_classified_then_waiting_states_are_not_inferred()
     {
        // Given: current fallback process evidence with supported client command names.
        let processes = [
            ProcessEvidence::live(11, "opencode"),
            ProcessEvidence::live(12, "codex"),
            ProcessEvidence::live(13, "claude"),
            ProcessEvidence::live(14, "gemini"),
        ];

        // When: fallback state classification runs.
        // Then: client-looking process evidence remains the existing non-shell Working fallback, never a waiting state.
        for process in processes {
            assert_eq!(classify_process(&process), AgentState::Working);
        }
    }

    fn record(identity: ProcessIdentity, basename: &str) -> Result<ProcessRecord, ModelError> {
        Ok(ProcessRecord::new(
            identity,
            Some(ProcessBasename::new(basename)?),
        ))
    }

    const fn record_without_basename(identity: ProcessIdentity) -> ProcessRecord {
        ProcessRecord::new(identity, None)
    }

    fn assert_known(
        metadata: &ProcessMetadata,
        identity: ProcessIdentity,
        basename: &str,
        kind: ClientKind,
    ) -> Result<(), ModelError> {
        assert_eq!(metadata.identity(), Some(&identity));
        assert_eq!(metadata.basename(), Some(&ProcessBasename::new(basename)?));
        assert_eq!(
            metadata.candidate(),
            ClientCandidate::known(kind, ClientConfidence::Low)
        );
        Ok(())
    }
}
