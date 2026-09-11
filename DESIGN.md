# Design system

agentmux is a quiet terminal command center: compact enough to scan quickly and stable enough to leave open all day. The interface prioritizes the selected agent's real terminal surface while keeping navigation and state visible.

## Layout

The screen has three persistent regions:

1. A compact header with product name, discovery summary, and degraded/input status.
2. A main region containing the project-grouped sidebar and one selected-agent preview.
3. A one-line, width-aware keyboard footer.

The sidebar is 28 columns in ordinary terminals and 22 columns below 80 columns. It may be toggled, but selecting an agent never hides it automatically. The preview always consumes all remaining main-region space. Header height and footer copy become more compact on narrow terminals.

## Sidebar

- Project headings use the sanitized workspace label and a subdued style.
- Agents appear in the same order used by keyboard navigation.
- `>` marks the selected row.
- Every row names the client and state; color reinforces but never replaces the state label.
- Selection crosses project boundaries without requiring a separate group action.

## Preview

- Exactly one agent pane is previewed at a time.
- The preview title contains the client and indicates selection or input focus.
- ANSI terminal styling is retained where safe.
- Plain content wraps at the available width.
- Terminal captures retain their native row layout and scroll to the newest visible content.
- Missing content has an explicit fallback instead of leaving an unexplained blank panel.

## Input and pane switching

Browsing mode is the default. `Tab`/`i` enters focused input mode and adds a visible banner, border treatment, and footer message. In this mode, keys go to the selected tmux pane; `Esc` returns to browsing.

`Enter`/`o` switches the current tmux client to the selected pane while agentmux remains alive. Tmux prefix + `A` returns to the dashboard pane.

## Color and emphasis

Use terminal-native colors so the UI respects the user's environment.

| Role | Treatment |
| --- | --- |
| Product/selection | Cyan and bold |
| Working | Yellow |
| Idle | Green |
| Waiting | Magenta or warning emphasis |
| Unknown/muted | Dark gray |
| Degraded/error | Red |
| Panel border | Dark gray; cyan when focused |

No meaning may depend on color alone. Avoid emoji and RGB-only decoration.

## Spacing and typography

- Assume a monospace terminal and measure layout in cells.
- Use one-cell horizontal padding where content needs separation from borders.
- Keep labels short and sentence case consistent.
- Prefer one-line controls and headings over decorative whitespace.
- Never let footer hints or metadata displace the live preview unnecessarily.

## Responsive behavior

- Recompute layout from the terminal area on every draw.
- Keep the sidebar narrow and give all remaining width to the preview.
- Reduce header and footer detail before sacrificing the preview.
- Avoid multi-agent grids in constrained space; focus remains on one full-area pane.
- Preserve access to the selected agent's current composer/status region by keeping the newest captured content.

## Accessibility and privacy

- Focus and state have textual indicators.
- All primary behavior is keyboard accessible.
- Help lists the active key bindings.
- Raw tmux session/window labels, argv, full paths, environments, and credentials do not appear in dashboard metadata.
- Pane captures remain local. Content is shown only for the selected pane and is not retained in daemon state.
