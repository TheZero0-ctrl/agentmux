# Product specification

## Purpose

agentmux is a local terminal dashboard for developers running multiple coding agents in tmux. It answers three questions without requiring the user to inspect every session manually:

1. Which agents are running?
2. What is each agent doing?
3. How can I reach the relevant pane quickly?

## Product principles

- **One place for every tmux session.** Discovery covers local sessions, windows, and panes rather than only the current tmux context.
- **One focused preview.** The selected agent receives the available main area, keeping rendering responsive and input unambiguous.
- **Conservative status.** Ambiguous evidence remains unknown; a live process alone does not prove active work.
- **Keyboard first.** Selection, preview input, pane switching, refresh, help, and sidebar visibility are available without a mouse.
- **Local and private.** Process evidence and pane content stay on the machine, and projected metadata is minimized.

## Supported environment

The current release targets Linux and tmux. It recognizes exact process basenames for:

- Codex (`codex`)
- OpenCode (`opencode` and `opencode.exe`)
- Claude Code (`claude`)
- Gemini CLI (`gemini`)

Command arguments are allowed. Generic shells, lookalike names, incomplete process trees, and conflicting evidence are not classified as supported agents.

## Dashboard behavior

The dashboard contains a project-grouped sidebar and one selected-agent preview.

- `j`/`k`, arrow keys, paging keys, and Home/End traverse the complete flattened agent list, including project boundaries.
- The selected preview fills all space beside the sidebar. Hiding the sidebar gives the preview the complete main area.
- Pane content is refreshed once per second and after relevant interactions.
- `Tab` or `i` enables explicit key forwarding to the selected pane; `Esc` returns to browsing mode.
- `Enter` or `o` switches the tmux client to the selected pane without terminating agentmux.
- tmux prefix + `A` returns to the dashboard while it remains running.
- A failed refresh preserves the last good rows and displays a degraded state.

The dashboard exits successfully without drawing when stdin or stdout is not an interactive terminal.

## State model

User-facing states include:

| State | Meaning |
| --- | --- |
| `working` | Current terminal evidence indicates active agent work. |
| `idle` | A known idle composer or completed interactive state is visible. |
| `waiting_permission` | The agent is waiting for a permission decision. |
| `waiting_plan_approval` | Structured evidence reports a plan approval request. |
| `waiting_question` | Structured evidence reports a question for the user. |
| `unknown` | Available evidence is missing, stale, conflicting, or unsupported. |
| `exited` | The backing pane or process is no longer live. |

Terminal markers refine Codex and OpenCode status in the dashboard. Structured daemon evidence can authoritatively provide waiting states. Two consecutive idle observations are required when transitioning from active work to reduce flicker caused by partial terminal redraws.

## Commands

- `agentmux` and `agentmux dashboard` open the interactive dashboard.
- `agentmux dashboard --no-daemon` disables daemon autostart.
- `agentmux inspect` prints one deterministic, privacy-safe TSV snapshot.
- `agentmux daemon --bind <address>` runs the local state service; non-loopback addresses are rejected.

## Privacy and security

- The daemon accepts loopback connections only.
- Project labels use a sanitized final path component.
- Raw session names, raw window names, argv, procfs command lines, environments, and credentials are excluded from projected rows.
- Pane captures are bounded and used locally for preview and terminal-state refinement. Only selected preview content is retained in the dashboard model, and pane text is not retained by the daemon.
- Input forwarding is off by default and scoped to the selected pane.
- Refresh and parsing errors are sanitized before display.

## Reliability requirements

- Discovery failure in one pane must not hide healthy agents in other panes.
- Preview capture failure must not remove an otherwise valid agent row.
- A daemon failure must fall back to local in-process discovery.
- Status changes must avoid oscillating during ordinary terminal redraws.
- Navigation order must match the project grouping shown in the sidebar.
- The UI must remain usable at narrow and wide terminal sizes without clipping the dashboard controls.

## Out of scope for the current release

- Non-tmux terminals and remote hosts
- Windows and macOS process discovery
- Persistent user configuration
- Search, pinning, and manual ordering
- Git, branch, pull-request, and CI metadata
- Guaranteed semantic status for unsupported client versions or themes
