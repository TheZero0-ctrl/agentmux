# agentmux

agentmux is the terminal dashboard for coding-agent workflows.

## Current capabilities

- CLI binary: `agentmux`
- Supported commands: `dashboard`, `inspect`
- Interactive surface: an empty-state Ratatui shell that shows `agentmux`, `No agents detected yet`, and `q quit`
- Outside an interactive terminal, the binary exits successfully without opening the shell
- Phase 2 tracer: daemon-owned, typed in-memory snapshots from strict read-only `tmux list-panes` output
- One-shot inspect output: `agentmux inspect` runs discovery once and prints normalized tmux-derived state
- Conservative fallback states for live shell, live command, stale, dead, and missing pane evidence

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
agent_id	pane_id	state	evidence_source	evidence_freshness	evidence_confidence
pane:%1	%1	idle	tmux	fresh	low
```

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

The roadmap is tracked in `docs/plan.md` and moves through these phases:

1. Foundation
2. tmux/process discovery + state
3. hooks/log adapters
4. daemon/SSE/reconciliation
5. preview/action
6. sidebar/search/grouping/pinning
7. Git/PR enrichment
8. hardening/distribution
