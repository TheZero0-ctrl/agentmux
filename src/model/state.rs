/// Normalized activity state for an agent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AgentState {
    /// The agent appears idle at a shell prompt.
    Idle,
    /// The agent pane has a live non-shell foreground process.
    Working,
    /// The agent is waiting for a permission response.
    WaitingPermission,
    /// The agent is waiting for plan approval.
    WaitingPlanApproval,
    /// The agent is waiting for a question response.
    WaitingQuestion,
    /// The fallback evidence cannot prove a more specific state.
    Unknown,
    /// The pane or process has exited.
    Exited,
}
