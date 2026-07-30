//! Generic process normalization for fallback discovery.

use crate::model::{AgentState, EvidenceFreshness, ProcessEvidence, ProcessLiveness};

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

/// Normalize a process command to its basename without login-shell prefix.
#[must_use]
pub fn normalize_command_name(command: &str) -> Option<&str> {
    let trimmed = command.trim();
    let basename = trimmed.rsplit('/').next()?;
    let without_login_prefix = basename
        .strip_prefix('-')
        .map_or(basename, |command_name| command_name);

    match without_login_prefix {
        "" => None,
        name => Some(name),
    }
}

fn is_shell_command(command: &str) -> bool {
    matches!(
        command,
        "sh" | "bash"
            | "zsh"
            | "fish"
            | "nu"
            | "dash"
            | "ksh"
            | "mksh"
            | "csh"
            | "tcsh"
            | "elvish"
            | "xonsh"
            | "pwsh"
            | "powershell"
    )
}
