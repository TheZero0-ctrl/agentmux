# agentmux

agentmux is the terminal dashboard for coding-agent workflows.

## Current capabilities

- CLI binary: `agentmux`
- Supported command: `dashboard`
- Interactive surface: an empty-state Ratatui shell that shows `agentmux`, `No agents detected yet`, and `q quit`
- Outside an interactive terminal, the binary exits successfully without opening the shell

## Commands

- `cargo build`
- `cargo test`
- `cargo run`
- `cargo run -- dashboard`
- `cargo run -- --help`

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
