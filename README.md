# agentmux

agentmux is the terminal dashboard for coding-agent workflows.

## Current capabilities

- CLI binary: `agentmux`
- Supported commands: `dashboard`, `inspect`
- Interactive surface: a Ratatui dashboard that renders the same normalized local discovery rows as `inspect`
- Outside an interactive terminal, the binary exits successfully without opening the shell
- Local discovery: strict read-only `tmux list-panes` collection plus Linux procfs process-tree evidence when available
- Linux-first candidate hints: exact executable basenames `opencode`, `codex`, `claude`, and `gemini` appear only as low-confidence candidates, never as authoritative identities
- Unknown semantics: missing, degraded, conflicting, generic, lookalike, or incomplete process evidence stays `unknown`
- Conservative states: live shell, live command, stale, dead, and missing pane evidence map to safe fallback states; waiting states are not inferred from tmux or procfs
- Dashboard refresh: synchronous in-process initial refresh, automatic refresh every second, manual `r` / `R` refresh, and quit controls `q`, Esc, and Ctrl-C
- Degraded behavior: refresh failures keep the last good dashboard rows and show a sanitized degraded message
- Privacy boundary: rows display sanitized workspace basenames and executable basenames only; full paths, argv, prompts, diffs, credentials, terminal content, and procfs cmdlines are not displayed or retained in presentation rows

## Commands

- `cargo build`
- `cargo test`
- `cargo run`
- `cargo run -- dashboard`
- `cargo run -- inspect`
- `cargo run -- --help`

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

- [Specification](docs/spec.md)
- [Architecture](docs/architecture.md)
- [Implementation plan](docs/plan.md)

## Roadmap

The roadmap is tracked in `docs/plan.md`. Shipped local discovery currently stops before network transport, daemon loops, authoritative adapters, previews/actions, Git/PR enrichment, search persistence, and cross-session persistence.

1. Foundation
2. tmux/Linux procfs discovery + state
3. hooks/log adapters
4. daemon/SSE/reconciliation
5. preview/action
6. sidebar/search/grouping/pinning
7. Git/PR enrichment
8. hardening/distribution
