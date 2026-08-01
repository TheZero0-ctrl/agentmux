# agentmux Specification

## Problem
Teams that use multiple coding agents need one place to see which local sessions may be active, what reliable evidence exists for them, and whether future branch or PR context needs attention. The current repo now ships local tmux plus Linux procfs discovery, a synchronous terminal dashboard, and a loopback live daemon slice for authoritative waiting-state evidence. It does not yet coordinate client-specific adapters, live previews, actions, persistence, search, Git/PR, or repository workflows.

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
- Treating a process basename as exact agent identity.
- Inferring waiting states from tmux, procfs, command names, terminal content, or process trees.
- Reading or displaying full command lines, prompts, diffs, credentials, environments, terminal content, or full workspace paths.
- Shipping persistence, Git/PR providers, previews, actions, search, terminal-content adapters, or client-specific authoritative identity adapters in the current milestone.

## Terms
- `agent`: one tracked coding assistant session from a supported client.
- `dashboard`: the main UI that lists agents, status, and repository context.
- `client candidate`: a low-confidence local hint from exact Linux procfs executable basename evidence, or `unknown`.
- `workspace label`: a privacy-safe final path component, or `unknown`; it is not a full path.
- `degraded`: a safe state shown when refresh or source evidence fails while preserving the last good rows where available.
- `live preview`: the detail panel that shows the currently selected agent or repo item.
- `act-in-place`: performing the next action from the dashboard instead of switching tools.
- `docked sidebar`: a side panel that resizes while the main content stays fixed.
- `event-first semantic update`: a state change derived from a meaningful event, not from polling alone.
- `reconciliation`: a pass that corrects local state from authoritative sources after events arrive.
- `adaptive preview polling`: preview refresh cadence that changes based on agent state and activity.

## Shipped Scope
The current implementation is:
- CLI startup.
- `dashboard`: a synchronous in-process Ratatui dashboard that performs an initial refresh, refreshes automatically every second, supports manual `r` / `R` refresh, and quits with `q`, Esc, or Ctrl-C.
- `inspect`: a one-shot local discovery command that runs discovery once and prints the shared normalized TSV row projection for agent rows only.
- Shared inspect/dashboard row fields: `agent_id`, `session_name`, `window_index`, `window_name`, `pane_id`, `pid`, `process_name`, `client`, `client_confidence`, `workspace`, `state`, `evidence_source`, `evidence_freshness`, and `evidence_confidence`.
- Strict read-only `tmux list-panes` parsing into typed session, window, pane, workspace label, and current-command evidence.
- Linux procfs process-tree discovery for PID plus start-time identity and executable basename evidence. Non-Linux or unavailable procfs evidence degrades safely instead of claiming support.
- Low-confidence client candidate classification for exact executable basenames `opencode`, `codex`, `claude`, and `gemini` only. Generic, lookalike, conflicting, missing, or degraded evidence remains `unknown`.
- Deterministic fallback agent rows for live shell, live command, stale, dead, missing, degraded, and empty snapshots.
- Degraded dashboard refresh handling that retains last good rows and shows a sanitized degraded message.
- `daemon`: a loopback-only local daemon that polls fallback discovery, ingests structured hook/marker/structured-log evidence, reconciles it in memory, and exposes `/health`, `/state`, `/events`, and `/evidence`.
- Authoritative waiting states from structured evidence only: `waiting_permission`, `waiting_plan_approval`, and `waiting_question`. Hook evidence outranks marker evidence, marker outranks structured-log evidence, higher sequence wins within a source, and `clear` removes an override. Exited or missing fallback panes win over waiting evidence.
- Dashboard daemon-first refresh: when the loopback daemon is reachable, dashboard rows come from daemon `/state`; otherwise the dashboard falls back to the existing in-process discovery path.
- Privacy-safe presentation: full paths, argv, procfs cmdlines, raw session/window labels, prompts, diffs, credentials, environments, terminal content, and non-agent panes are not displayed in rows. Session and window names render as `unknown`; numeric window indexes remain available because they are parsed typed metadata.

Everything below remains **Planned — not implemented yet**.

## Planned — not implemented yet

### Agent Clients
- Authoritative Claude Code adapter.
- Authoritative Codex adapter.
- Authoritative Cursor adapter.
- Authoritative OpenCode adapter.
- Authoritative Pi adapter.
- Authoritative Gemini CLI adapter.
- Configured custom agent adapters.

The shipped basename hints for `opencode`, `codex`, `claude`, and `gemini` are only low-confidence local candidates. They do not implement authoritative client identity.

### Agent States
- `idle`: shipped for live shell fallback evidence.
- `working`: shipped for live non-shell fallback evidence.
- `unknown`: shipped for stale or incomplete fallback evidence.
- `exited`: shipped for dead or missing pane fallback evidence.
- `waiting_permission`: shipped for structured authoritative hook, marker, or structured-log evidence.
- `waiting_plan_approval`: shipped for structured authoritative hook, marker, or structured-log evidence.
- `waiting_question`: shipped for structured authoritative hook, marker, or structured-log evidence.

### Dashboard Capabilities
- Repository context beyond privacy-safe workspace labels.
- Live preview for the selected agent, branch, worktree, or PR.
- Act-in-place responses from the focused row or panel.
- Docked sidebar resizing without moving the main layout.
- Fuzzy search across agents, repos, branches, and reviews.
- Grouping by client, repository, branch, or status.
- Pinning important agents or workspaces.
- Reordering items by user preference.
- Branch, worktree, and PR CI review status.

### Update Model
- Event-first semantic updates for structured local evidence.
- In-memory reconciliation after structured evidence delivery.
- Adaptive preview polling tied to current activity.

### Discovery Beyond One-Shot Inspect
- Cross-platform procfs-equivalent process discovery remains unimplemented.
- Cross-platform procfs-equivalent process discovery remains unimplemented.
- Process-tree discovery beyond the shipped Linux procfs basename hints.
- Terminal-content adapters remain unimplemented.

### Security
- Keep secrets out of the visible dashboard surface.
- Avoid exposing full prompts, private diffs, or credentials by default.
- Require explicit user action for sensitive operations.

### Privacy
- Keep local agent context local unless the user chooses otherwise.
- Minimize captured metadata to what the dashboard needs.
- Make privacy boundaries visible in the UI.
- Display sanitized workspace labels instead of full paths.
- Do not read procfs cmdlines, environments, prompts, diffs, credentials, or terminal content for the shipped local discovery row.

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
- The CLI starts and opens the synchronous local discovery dashboard in an interactive terminal.
- `agentmux inspect` runs one local discovery pass and prints deterministic normalized output with the shipped enriched header.
- Empty inspect snapshots explain that no agents were discovered from tmux panes.
- tmux discovery failures exit nonzero without partial inspect stdout.
- Dashboard tests cover the documented controls and degraded refresh behavior.
- Low-confidence Linux procfs candidate semantics and privacy boundaries are documented as shipped behavior.
- The spec clearly separates shipped scope from planned capabilities.
- Every later capability in this document is labeled `Planned — not implemented yet`.
- The document includes problem, users, goals, non-goals, and terms.
- The document names Claude Code, Codex, Cursor, OpenCode, Pi, Gemini CLI, and custom agents.
- The document names all required agent states.
- The document names dashboard, live preview, act-in-place, docked resizing sidebar, fuzzy search, grouping, pinning, reorder, and branch/worktree/PR CI review status.
- The document names event-first semantic updates, reconciliation, and adaptive preview polling.
- The document states security, privacy, performance, and accessibility expectations.
