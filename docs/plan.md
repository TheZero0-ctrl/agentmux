# agentmux Implementation Plan

This plan is derived from the approved spec and architecture docs. The foundation, local discovery dashboard, and loopback daemon state/API vertical slice are already in place; future phases remain below.

## Phase 1: Foundation

- Depends on: approved spec, approved architecture, current CLI/TUI foundation.
- Entry criteria: `agentmux` starts, parses the CLI, and renders the terminal dashboard in an interactive terminal.
- Exit criteria: the foundation surface is stable, documented, and covered by CLI and TUI tests.
- Tests: `cargo test`, CLI help/command contract tests, dashboard render tests, and non-interactive dashboard exit tests.
- Risks: scope creep into orchestration work too early, and README/help text drifting from the actual binary.

## Phase 2: tmux/process discovery + state

- Status: implemented as a local discovery milestone. It includes typed fallback state, strict
  `tmux list-panes` parsing, Linux procfs process-tree evidence, exact-basename low-confidence
  candidates for `opencode`, `codex`, `claude`, and `gemini`, privacy-safe workspace labels,
  in-memory missing/stale handling, a shared filtered inspect/dashboard agent-row projection, a shipped one-shot
  `agentmux inspect` command, and a synchronous in-process Ratatui dashboard refresh loop.
  Cross-platform process discovery, client-specific authoritative adapters, persistence,
  previews/actions, search, and Git / PR enrichment remain future work.
- Depends on: foundation shell, daemon boundary, and a state model that can represent live processes and panes.
- Entry criteria: the project can observe tmux panes and related process evidence without UI code reaching into tmux directly.
- Exit criteria: tmux and Linux procfs evidence feed a structured state model; `agentmux inspect`
  exposes one deterministic enriched snapshot; the dashboard renders the same projection and refreshes
  synchronously with documented controls and degraded behavior.
- Tests: unit tests for pane/process discovery, procfs fixtures, classifier contracts, state translation,
  stale or missing panes, inspect seam tests, dashboard runner/TUI tests, and CLI inspect failure tests.
- Risks: tmux attachment edge cases, Linux procfs permission/race behavior, and noisy process trees causing
  false confidence if future code treats basename candidates as exact identity.

## Phase 3: hooks/log adapters

- Status: partially implemented. Structured hook, marker, and structured-log evidence can assert waiting states through the daemon `/evidence` endpoint. Client-specific adapters, log replay, and exact identity remain future.
- Depends on: process/state model, source precedence rules, and a collector boundary for external evidence.
- Entry criteria: hook events, markers, and structured logs have a place to land in the state pipeline.
- Exit criteria: hooks and log adapters produce normalized events that can be ranked alongside other evidence. The current slice ships the normalized ingestion and precedence core, not adapter-specific collectors.
- Tests: adapter parsing tests, precedence tests for conflicting sources, fixture-based log replay tests.
- Risks: log format drift, incomplete adapter coverage, and incorrect precedence when sources disagree.

## Phase 4: daemon/SSE/reconciliation

- Status: partially implemented. The shipped daemon binds only to loopback, exposes `/health`, `/state`, `/events`, and `/evidence`, folds structured evidence in memory, and the dashboard falls back to in-process discovery when the daemon is unavailable. Persistence, authentication, multi-client transport hardening, and rich reconciliation remain future.
- Depends on: normalized event ingestion, state reducer, and authoritative-source precedence.
- Entry criteria: the daemon can accept commands and publish semantic updates from the reducer.
- Exit criteria: loopback HTTP plus SSE surface live state, and reconciliation corrects stale or ambiguous state.
- Tests: daemon request/response tests, SSE stream tests, reconciliation tests against authoritative fixtures.
- Risks: reconnect churn, stale state after missed events, and over-reliance on polling.

## Phase 5: preview/action

- Status: partially implemented. The shipped dashboard has a toggleable project-grouped agent sidebar, a responsive single-agent main preview, local tmux pane-text capture, and focused-pane keyboard forwarding behind explicit `i` input mode. Richer act-in-place actions remain future.
- Depends on: daemon API, live state selection, and a defined action boundary.
- Entry criteria: the UI can select an item and ask for a detail preview or a scoped action. Sidebar selection and the focused agent preview are shipped.
- Exit criteria: live agent content is shown for focused items, and act-in-place actions reach the daemon. Both real content and action halves remain future.
- Tests: action dispatch tests, preview rendering tests, failure-path tests for missing or stale selections.
- Risks: leaking private content into the preview, ambiguous focus handling, and action surfaces that drift from state.

## Phase 6: sidebar/search/grouping/pinning

- Status: future. The shipped dashboard renders responsive local rows but does not include sidebar resizing, search, grouping, pinning, or reorder.
- Depends on: stable selection state, persisted search data, and layout primitives that can resize safely.
- Entry criteria: the dashboard has a docked sidebar target and a searchable collection of rows/items.
- Exit criteria: sidebar width changes independently, search works, items can be grouped, pinned, and reordered.
- Tests: layout regression tests, fuzzy-search tests, grouping/pinning/reorder state tests.
- Risks: layout jitter, search index staleness, and ordering bugs that confuse the dashboard.

## Phase 7: Git/PR enrichment

- Status: future. The shipped rows include privacy-safe workspace labels only, not branch, worktree, PR, or CI review status.
- Depends on: repository/worktree identity, cached provider plumbing, and bounded external lookups.
- Entry criteria: the app can resolve branch, worktree, PR, and CI review context for a repository item.
- Exit criteria: Git/PR metadata appears in the dashboard and remains cached, bounded, and time-limited.
- Tests: provider cache tests, repository-resolution tests, timeout and stale-cache regression tests.
- Risks: slow provider calls, inconsistent branch/worktree mapping, and stale review status.

## Phase 8: Hardening/distribution

- Depends on: all prior phases having stable contracts and coverage.
- Entry criteria: the main flows are implemented and the remaining work is mostly polish, performance, and packaging.
- Exit criteria: the binary is dependable, the UI is accessible and responsive, and the docs describe the shipped surface accurately.
- Tests: full `cargo test`, targeted stress and regression tests, manual terminal QA for the interactive shell.
- Risks: late-stage regressions, performance cliffs under many agents, and documentation drifting from shipped behavior.
