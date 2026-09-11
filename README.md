# agentmux

agentmux is the terminal dashboard for coding-agent workflows.

## Current capabilities

- CLI binary: `agentmux`
- Supported commands: `dashboard`, `inspect`, `daemon`
- Interactive surface: a responsive Ratatui dashboard that auto-starts a missing local daemon, prefers daemon state, and shows the same filtered agent rows as `inspect`
- Outside an interactive terminal, the binary exits successfully without opening the shell
- Local discovery: strict read-only `tmux list-panes` collection plus Linux procfs process-tree evidence when available; presentation filters out non-agent panes
- Linux-first candidate hints: exact executable basenames `opencode`, `codex`, `claude`, and `gemini` appear only as low-confidence candidates, never as authoritative identities
- Unknown semantics: missing, degraded, conflicting, generic, lookalike, or incomplete process evidence stays `unknown`
- Conservative states: live shell, live command, stale, dead, and missing pane evidence map to safe fallback states; waiting states are accepted only from structured hook, marker, or structured-log evidence
- Live daemon: `agentmux daemon` binds to loopback only, accepts structured evidence, overlays authoritative waiting states onto fallback discovery, and exposes privacy-safe `/health`, `/state`, `/events`, and `/evidence`
- Dashboard refresh and navigation: synchronous in-process initial refresh, automatic refresh every second, manual `r` / `R` refresh, row selection with `j` / `k`, arrows, Home/End, PageUp/PageDown, project-grouped sidebar with `s`, help with `?` / `h`, live preview input focus with `Tab` / `i`, switching to the selected real tmux pane with `Enter` / `o`, and quit controls `q`, Esc, and Ctrl-C
- Live preview: the selected agent renders its current tmux pane text, refreshed with the dashboard projection; unavailable panes show an empty-content fallback without degrading discovery. Press `i` to forward keyboard input to the focused pane, `o` to switch the tmux client to the focused agent's real pane while leaving agentmux running, `tmux prefix + A` to return to agentmux, and `Esc` to leave input mode.
- Degraded behavior: refresh failures keep the last good dashboard rows and show a sanitized degraded message
- Privacy boundary: rows display sanitized workspace basenames and executable basenames only; full paths, argv, credentials, and procfs cmdlines are not displayed or retained in presentation rows. Pane text is captured only for visible local tiles and is not retained in daemon state.

## Commands

- `cargo build`
- `cargo test`
- `cargo run`
- `cargo run -- dashboard`
- `cargo run -- dashboard --no-daemon`
- `cargo run -- inspect`
- `cargo run -- daemon --bind 127.0.0.1:47631`
- `cargo run -- --help`

## Dashboard

`agentmux dashboard` opens a local dashboard with a header/status area, responsive main area, and compact footer controls.

- Wide terminals show a privacy-safe agent list plus a detail panel for the selected row.
- Medium terminals show a single list with safe location, workspace, process, and evidence metadata.
- Narrow terminals collapse to compact essentials: client, state, and safe pane location.
- Empty and degraded states are explicit; degraded refreshes keep the last good rows.
- Selection follows keyboard navigation and scrolls the visible list when the selected row moves outside the viewport.
- The sidebar lists discovered agents grouped by sanitized project/workspace name. `s` toggles the sidebar; `j` / `k` selection controls the single full-area live preview.

The dashboard does not render raw tmux session names, raw window names, argv, full paths, credentials, or raw command stderr. The selected tmux pane text is rendered in the full-area preview.

In an interactive terminal, `agentmux dashboard` checks `GET /health` on `127.0.0.1:47631`. If a healthy daemon is already running, the dashboard uses it without owning or stopping it. If the endpoint is unreachable, the dashboard starts the current executable as `agentmux daemon --bind 127.0.0.1:47631`, owns only that child process, and stops/reaps that child when the dashboard exits. If the endpoint is occupied by an invalid listener, the dashboard does not spawn over it.

`agentmux dashboard --no-daemon` skips daemon autostart. In all daemon-unavailable or invalid cases, the dashboard uses the existing in-process discovery path, filters out non-agent panes, and keeps the successful no-output behavior outside an interactive terminal.

## Daemon API

The daemon is a local-only vertical slice. It rejects non-loopback bind addresses and uses a small standard-library HTTP/SSE subset:

- `GET /health`: plain text daemon health with the current revision.
- `GET /state`: privacy-safe TSV beginning with `agentmux state`, `revision: <u64>`, `agents: <N>`, then the same projection header as `inspect`.
- `GET /events`: `text/event-stream`; sends the current snapshot immediately and then snapshot events when revisions change.
- `POST /evidence`: newline-delimited structured evidence, maximum 4096 bytes.

Accepted evidence lines look like:

```text
agentmux.v1 source=hook agent_id=pane:%1 state=waiting_permission sequence=42
```

`source` is `hook`, `marker`, or `structured_log`. `state` is `waiting_permission`, `waiting_plan_approval`, `waiting_question`, or `clear`. Hook evidence outranks marker evidence, marker outranks structured-log evidence, and higher sequence wins within one source. `clear` removes an override. Missing or exited fallback panes still win over waiting evidence.

`agentmux inspect` prints a deterministic table when panes are discovered:

```text
agentmux inspect
agents: 1
agent_id	session_name	window_index	window_name	pane_id	pid	process_name	client	client_confidence	workspace	state	evidence_source	evidence_freshness	evidence_confidence
pane:%1	unknown	0	unknown	%1	1234	opencode	opencode	low	agentmux	working	tmux	fresh	low
```

The `session_name` and `window_name` columns render as `unknown`; raw tmux labels can contain prompts, tokens, diffs, or other sensitive text.

The `client` column is a low-confidence local candidate derived from an exact executable basename on Linux procfs evidence. It is `unknown` when evidence is missing, conflicting, unsupported, generic, unavailable, or not Linux procfs-backed.

When no tmux panes are discovered, it prints:

```text
agentmux inspect
agents: 0
no agents discovered from tmux panes
```

## Docs

- [Design system](DESIGN.md)
- [Specification](docs/spec.md)
- [Architecture](docs/architecture.md)
- [Implementation plan](docs/plan.md)
- [Release process](docs/releasing.md)

## Roadmap

The roadmap is tracked in `docs/plan.md`. Shipped local discovery now includes the loopback daemon state/API vertical slice, project-grouped agent sidebar, focused main preview, and local tmux pane-text capture. Act-in-place actions, search, Git/PR enrichment, cross-platform discovery, adapter-specific identity, durable persistence, and cross-session persistence remain future work.

1. Foundation
2. tmux/Linux procfs discovery + state
3. hooks/log adapters
4. daemon/SSE/reconciliation
5. preview/action
6. sidebar/search/grouping/pinning
7. Git/PR enrichment
8. hardening/distribution
