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

## Shipped Scope
The current implementation is:
- CLI startup.
- `dashboard`: an interactive empty-state TUI shell.
- `inspect`: a one-shot tmux-derived tracer that runs discovery once and prints normalized state.
- Strict read-only `tmux list-panes` parsing into typed pane and process evidence.
- Deterministic fallback agent rows for live shell, live command, stale, dead, missing, and empty snapshots.

Everything below remains **Planned — not implemented yet**.

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
- `idle`: shipped for live shell fallback evidence.
- `working`: shipped for live non-shell fallback evidence.
- `unknown`: shipped for stale or incomplete fallback evidence.
- `exited`: shipped for dead or missing pane fallback evidence.
- `waiting_permission`: planned for authoritative hook/log evidence.
- `waiting_plan_approval`: planned for authoritative hook/log evidence.
- `waiting_question`: planned for authoritative hook/log evidence.

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

### Discovery Beyond One-Shot Inspect
- Long-running daemon loop.
- Polling or evented reconciliation.
- Process-tree discovery beyond the current tmux current-command fallback.
- Hooks, markers, structured logs, and terminal-content adapters.
- HTTP/SSE transport for live UI rows.

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
- `agentmux inspect` runs one tmux discovery pass and prints deterministic normalized output.
- Empty inspect snapshots explain that no agents were discovered from tmux panes.
- tmux discovery failures exit nonzero without partial inspect stdout.
- The spec clearly separates shipped scope from planned capabilities.
- Every later capability in this document is labeled `Planned — not implemented yet`.
- The document includes problem, users, goals, non-goals, and terms.
- The document names Claude Code, Codex, Cursor, OpenCode, Pi, Gemini CLI, and custom agents.
- The document names all required agent states.
- The document names dashboard, live preview, act-in-place, docked resizing sidebar, fuzzy search, grouping, pinning, reorder, and branch/worktree/PR CI review status.
- The document names event-first semantic updates, reconciliation, and adaptive preview polling.
- The document states security, privacy, performance, and accessibility expectations.
