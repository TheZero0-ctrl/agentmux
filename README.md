# agentmux

Monitor and control coding agents across all of your local tmux sessions from one terminal dashboard.

agentmux discovers supported agent processes, groups them by project, shows the selected pane live, and lets you jump to or type into that pane without losing the dashboard.

## Features

- Discovers agents across tmux sessions, windows, and panes.
- Recognizes Codex, OpenCode, Claude Code, and Gemini CLI processes.
- Groups agents by project in a keyboard-navigable sidebar.
- Shows one responsive, full-size live pane preview at a time.
- Reports `working`, `idle`, `waiting_permission`, `unknown`, and `exited` states conservatively.
- Forwards input to the selected pane only after entering input mode.
- Switches the current tmux client to an agent pane and keeps agentmux running.
- Keeps process metadata private and binds its optional daemon to loopback only.

## Requirements

- Linux
- tmux
- A terminal with color and keyboard input support

Building from source also requires Rust 1.96 or newer.

## Installation

Download the Linux x86_64 archive and checksum from the [latest GitHub release](https://github.com/TheZero0-ctrl/agentmux/releases/latest), extract it, and place `agentmux` somewhere on your `PATH`.

To build from source:

```sh
git clone https://github.com/TheZero0-ctrl/agentmux.git
cd agentmux
cargo build --locked --release
install -Dm755 target/release/agentmux ~/.local/bin/agentmux
```

## Quick start

Run agentmux from inside tmux:

```sh
agentmux
```

The default command opens the dashboard. Start Codex, OpenCode, Claude Code, or Gemini CLI in any local tmux pane; discovered agents appear automatically.

Use `j` and `k` to select an agent. Press `Enter` to switch to its real tmux pane. While agentmux is running, press your tmux prefix followed by `A` to return to the dashboard pane.

## Keyboard controls

| Key | Action |
| --- | --- |
| `j` / `Down` | Select next agent |
| `k` / `Up` | Select previous agent |
| `Home` / `End` | Select first or last agent |
| `PageUp` / `PageDown` | Move through the list by page |
| `Tab` / `i` | Enter input mode for the selected pane |
| `Esc` | Leave input mode |
| `Enter` / `o` | Switch to the selected tmux pane |
| tmux prefix + `A` | Return to the agentmux pane |
| `s` | Toggle the sidebar |
| `r` | Refresh immediately |
| `?` / `h` | Toggle help |
| `q` / `Ctrl-C` | Quit |

In input mode, keyboard events are sent directly to the selected agent pane. Leave input mode with `Esc` before using dashboard shortcuts.

## Commands

```text
agentmux                         Open the dashboard
agentmux dashboard              Open the dashboard explicitly
agentmux dashboard --no-daemon  Do not start a local daemon automatically
agentmux inspect                Print one discovery snapshot as TSV
agentmux daemon                 Run the local state daemon
agentmux --help                 Show CLI help
```

The dashboard starts a local daemon on `127.0.0.1:47631` when one is not already available. If the daemon cannot be used, discovery falls back to the in-process path. `--no-daemon` uses an existing daemon when available but never starts one.

## Agent status

agentmux combines tmux pane data, Linux process-tree evidence, and current terminal state. A running client process is not automatically treated as active: known composer and interrupt markers are used to distinguish idle and working sessions. Brief ambiguous transitions are stabilized to avoid status flicker.

Waiting states may also be supplied as structured evidence to the local daemon. Unknown or incomplete evidence stays `unknown` instead of being presented as certainty.

## Discovery and privacy

Discovery is local and spans every pane returned by `tmux list-panes -a`. Process inspection reads Linux procfs and matches exact supported executable names, including launchers with arguments and `.exe` suffixes.

The sidebar uses sanitized project labels. Raw tmux session/window names, full command lines, environments, credentials, and full paths are excluded from projected rows. Bounded pane captures refine live status; only the selected pane's content is retained in the dashboard model for preview, and pane text is never retained in daemon state.

## Troubleshooting

If an agent is missing, verify that it is running in tmux and that its foreground process resolves to `codex`, `opencode`, `claude`, or `gemini`. Then press `r` or compare discovery output with:

```sh
agentmux inspect
tmux list-panes -a -F '#{pane_id} #{pane_current_command} #{pane_current_path}'
```

If switching works but tmux prefix + `A` does not return to the dashboard, make sure agentmux itself was started inside a tmux pane.

## Documentation

- [Product specification](docs/spec.md)
- [Architecture](docs/architecture.md)
- [Development roadmap](docs/plan.md)
- [Design system](DESIGN.md)
- [Release guide](docs/releasing.md)

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --locked --release
```
