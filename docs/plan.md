# agentmux Implementation Plan

This plan is derived from the approved spec and architecture docs. The foundation is already in place; everything below is the forward roadmap.

## Phase 1: Foundation

- Depends on: approved spec, approved architecture, current CLI/TUI foundation.
- Entry criteria: `agentmux` starts, parses the CLI, and renders the empty-state TUI in an interactive terminal.
- Exit criteria: the foundation surface is stable, documented, and covered by CLI and TUI tests.
- Tests: `cargo test`, CLI help/command contract tests, empty-state render test.
- Risks: scope creep into orchestration work too early, and README/help text drifting from the actual binary.

## Phase 2: tmux/process discovery + state

- Status: the first tracer is implemented with typed fallback state, strict `tmux list-panes`
  parsing, conservative command classification, in-memory missing/stale handling, and a shipped
  one-shot `agentmux inspect` command. Broader process-tree discovery, polling, live UI rows, and
  integration into a long-running daemon remain future work.
- Depends on: foundation shell, daemon boundary, and a state model that can represent live processes and panes.
- Entry criteria: the project can observe tmux panes and related process evidence without UI code reaching into tmux directly.
- Exit criteria: tmux current-command discovery feeds a structured state model and `agentmux inspect`
  exposes one deterministic one-shot snapshot.
- Tests: unit tests for pane/process discovery, fixtures for state translation, regression tests for stale or missing panes, inspect seam tests, and CLI inspect failure tests.
- Risks: tmux attachment edge cases, permission issues, and noisy process trees causing false state changes.

## Phase 3: hooks/log adapters

- Depends on: process/state model, source precedence rules, and a collector boundary for external evidence.
- Entry criteria: hook events, markers, and structured logs have a place to land in the state pipeline.
- Exit criteria: hooks and log adapters produce normalized events that can be ranked alongside other evidence.
- Tests: adapter parsing tests, precedence tests for conflicting sources, fixture-based log replay tests.
- Risks: log format drift, incomplete adapter coverage, and incorrect precedence when sources disagree.

## Phase 4: daemon/SSE/reconciliation

- Depends on: normalized event ingestion, state reducer, and authoritative-source precedence.
- Entry criteria: the daemon can accept commands and publish semantic updates from the reducer.
- Exit criteria: loopback HTTP plus SSE surface live state, and reconciliation corrects stale or ambiguous state.
- Tests: daemon request/response tests, SSE stream tests, reconciliation tests against authoritative fixtures.
- Risks: reconnect churn, stale state after missed events, and over-reliance on polling.

## Phase 5: preview/action

- Depends on: daemon API, live state selection, and a defined action boundary.
- Entry criteria: the UI can select an item and ask for a detail preview or a scoped action.
- Exit criteria: preview content is shown for the focused item, and act-in-place actions reach the daemon.
- Tests: action dispatch tests, preview rendering tests, failure-path tests for missing or stale selections.
- Risks: leaking private content into the preview, ambiguous focus handling, and action surfaces that drift from state.

## Phase 6: sidebar/search/grouping/pinning

- Depends on: stable selection state, persisted search data, and layout primitives that can resize safely.
- Entry criteria: the dashboard has a docked sidebar target and a searchable collection of rows/items.
- Exit criteria: sidebar width changes independently, search works, items can be grouped, pinned, and reordered.
- Tests: layout regression tests, fuzzy-search tests, grouping/pinning/reorder state tests.
- Risks: layout jitter, search index staleness, and ordering bugs that confuse the dashboard.

## Phase 7: Git/PR enrichment

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
