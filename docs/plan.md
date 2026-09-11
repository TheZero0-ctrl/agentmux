# Development roadmap

This roadmap describes work beyond the current local tmux dashboard. Shipped behavior is documented in the [README](../README.md) and [product specification](spec.md).

## Shipped in 0.1

- Cross-session tmux pane discovery on Linux
- Process-tree detection for Codex, OpenCode, Claude Code, and Gemini CLI
- Project-grouped sidebar with complete keyboard traversal
- Single responsive live pane preview
- Explicit input forwarding and tmux pane switching
- Temporary tmux shortcut for returning to the dashboard
- Terminal-aware Codex and OpenCode status refinement
- Loopback daemon with state, event, health, and evidence endpoints
- Privacy-safe inspect output and degraded fallback behavior
- CI validation and tag-driven Linux x86_64 releases

## Near term

- Expand fixture coverage for supported client versions and terminal widths.
- Improve status adapters without treating historical pane text as current state.
- Add stress tests for many sessions, panes, and rapid process transitions.
- Preserve and restore any pre-existing tmux return binding.
- Add packaged installation options beyond GitHub release archives.

## Later

- Configurable agent executable aliases
- Search, filtering, pinning, and ordering
- Optional repository, branch, pull-request, and CI context
- Durable daemon state where it improves startup or recovery
- macOS and other platform-specific process collectors
- Authenticated remote or multi-host coordination

## Release quality bar

Every release must:

- Pass formatting, Clippy, tests, release build, and package verification.
- Keep discovery and preview failures isolated per pane.
- Preserve privacy boundaries in inspect and daemon output.
- Document user-visible controls and platform requirements accurately.
- Include regression coverage for detection, status, navigation, and responsive rendering changes.
