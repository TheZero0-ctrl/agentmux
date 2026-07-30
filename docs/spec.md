# agentmux Specification

## Problem
Teams that use multiple coding agents need one place to see which agent is active, what it is waiting on, and whether a branch or PR needs attention. The foundation in this repo only proves the CLI entrypoint and a minimal interactive empty-state TUI; it does not yet coordinate real agents, live previews, or repository workflows.

## Users
- Individual developers juggling Claude Code, Codex, Cursor, OpenCode, Pi, Gemini CLI, and custom configured agents.
- Small teams that want a shared dashboard for agent activity, review status, and pending approvals.
- Power users who want fast keyboard-driven control without losing track of worktrees, branches, or PR checks.

## Goals
- Provide a single dashboard for agent activity across Claude Code, Codex, Cursor, OpenCode, Pi, Gemini CLI, and custom agents.
- Show clear state for each agent: `idle`, `working`, `waiting(permission)`, `waiting(plan approval)`, `waiting(question)`, `unknown`, and `exited`.
- Support a live preview panel for the selected agent or repository item.
- Support act-in-place actions from the dashboard so users can respond without leaving the view.
- Support a docked, resizable sidebar that changes width without moving the main layout.
- Support fuzzy search, grouping, pinning, and reorder of agents and workspaces.
- Surface branch, worktree, PR, and CI review status in one view.
- Use event-first semantic updates, then reconcile state, then adapt preview polling to the current activity.
- Keep the experience secure, private, performant, and accessible.

## Non-Goals
- Replacing the user’s agent clients.
- Editing agent internals or simulating agent output.
- Building the full orchestration layer in the foundation.
- Persisting cross-session history beyond what the foundation needs.
- Adding every planned capability now.

## Terms
- `agent`: one tracked coding assistant session from a supported client.
- `dashboard`: the main UI that lists agents, status, and repository context.
- `live preview`: the detail panel that shows the currently selected agent or repo item.
- `act-in-place`: performing the next action from the dashboard instead of switching tools.
- `docked sidebar`: a side panel that resizes while the main content stays fixed.
- `event-first semantic update`: a state change derived from a meaningful event, not from polling alone.
- `reconciliation`: a pass that corrects local state from authoritative sources after events arrive.
- `adaptive preview polling`: preview refresh cadence that changes based on agent state and activity.

## Foundation Scope
The current implementation is only:
- CLI startup.
- An interactive empty-state TUI shell.

Everything below is **Planned — not implemented yet**.

## Planned — not implemented yet

### Agent Clients
- Claude Code
- Codex
- Cursor
- OpenCode
- Pi
- Gemini CLI
- Configured custom agents

### Agent States
- Idle
- Working
- Waiting on permission
- Waiting on plan approval
- Waiting on a question reply
- Unknown
- Exited

### Dashboard Capabilities
- Main dashboard with agent rows and repository context.
- Live preview for the selected agent, branch, worktree, or PR.
- Act-in-place responses from the focused row or panel.
- Docked sidebar resizing without moving the main layout.
- Fuzzy search across agents, repos, branches, and reviews.
- Grouping by client, repository, branch, or status.
- Pinning important agents or workspaces.
- Reordering items by user preference.
- Branch, worktree, and PR CI review status.

### Update Model
- Event-first semantic updates.
- Reconciliation after event delivery.
- Adaptive preview polling tied to current activity.

### Security
- Keep secrets out of the visible dashboard surface.
- Avoid exposing full prompts, private diffs, or credentials by default.
- Require explicit user action for sensitive operations.

### Privacy
- Keep local agent context local unless the user chooses otherwise.
- Minimize captured metadata to what the dashboard needs.
- Make privacy boundaries visible in the UI.

### Performance
- Keep startup fast.
- Keep list rendering responsive with many agents.
- Avoid unnecessary polling when agents are idle.
- Prefer incremental updates over full refreshes.

### Accessibility
- Keyboard-first navigation.
- Clear focus handling.
- High-contrast readable status indicators.
- Screen-reader-friendly labels where the terminal stack allows them.

## Measurable Acceptance
- The foundation starts as a CLI and opens the interactive empty-state TUI.
- The spec clearly separates current foundation scope from planned capabilities.
- Every later capability in this document is labeled `Planned — not implemented yet`.
- The document includes problem, users, goals, non-goals, and terms.
- The document names Claude Code, Codex, Cursor, OpenCode, Pi, Gemini CLI, and custom agents.
- The document names all required agent states.
- The document names dashboard, live preview, act-in-place, docked resizing sidebar, fuzzy search, grouping, pinning, reorder, and branch/worktree/PR CI review status.
- The document names event-first semantic updates, reconciliation, and adaptive preview polling.
- The document states security, privacy, performance, and accessibility expectations.
