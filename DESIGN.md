# agentmux Design System

## 1. Atmosphere & Identity

agentmux should feel like a quiet terminal command center for local coding-agent work: dense enough to be useful, calm enough to stay open all day. The signature is a ccmux-inspired session picker: a compact list of privacy-safe agent rows, a wide-screen detail panel, and a footer that always tells the operator what can be done now.

## 2. Color

### Palette

| Role | Token | Terminal color | Usage |
|------|-------|----------------|-------|
| Surface/base | `base` | default background | Main terminal background |
| Surface/panel | `panel` | black | Header, list, and detail panel blocks |
| Text/primary | `text` | white | Primary labels and selected content |
| Text/secondary | `subtext` | gray | Hints, metadata, and footer text |
| Border/default | `border` | dark gray | Panel borders and separators |
| Accent/active | `active` | cyan | Active row marker and focused shell title |
| Status/working | `working` | yellow | Active work state |
| Status/idle | `idle` | green | Calm idle state |
| Status/unknown | `unknown` | dark gray | Unknown or unavailable state |
| Status/degraded | `degraded` | red | Refresh degradation and warning status |

### Rules

- Use semantic terminal colors only; do not add new dependencies or RGB-only styling for this branch.
- Color must reinforce status, not carry information by itself. Text labels remain explicit.
- Raw tmux names, terminal content, prompts, diffs, argv, full paths, credentials, and command stderr never appear in the UI.

## 3. Typography

### Scale

| Level | Style | Usage |
|-------|-------|-------|
| Title | bold | `agentmux` shell title |
| Label | bold or dim | Panel titles and field labels |
| Body | normal | Row content and detail values |
| Metadata | dim | Evidence, hints, and privacy notes |

### Font Stack

- Primary: terminal default monospace.
- Mono: terminal default monospace.

### Rules

- Assume a monospace terminal; align with layout constraints rather than proportional typography.
- Keep row labels short and scannable.
- Do not use emoji glyphs as icons or status indicators.

## 4. Spacing & Layout

### Base Unit

Terminal spacing is measured in cells. One cell is the base unit.

| Token | Value | Usage |
|-------|-------|-------|
| `gap-1` | 1 cell | Cell separation and footer hint separators |
| `pad-x` | 1 cell | Panel inner horizontal padding |
| `row-main` | 1 line | Compact row summary |
| `row-detail` | 1 line | Optional metadata row |

### Grid

- Wide: `>= 137` columns, header/status + list/detail split + footer.
- Medium: `80..136` columns, header/status + single list + footer.
- Narrow: `< 80` columns, header/status + compact list + short footer.

### Rules

- The renderer derives layout from `frame.area()` every draw.
- Wide mode may show a right-side detail panel for the selected row only.
- Medium and narrow modes never show the wide detail panel.
- Footer hints are width-aware and drop lower-priority text instead of overflowing.

## 5. Components

### Shell

- **Structure**: header/status zone, main zone, footer zone.
- **Variants**: wide, medium, narrow.
- **Spacing**: `gap-1`, `pad-x`.
- **States**: normal, empty, degraded.
- **Accessibility**: every colored state has visible text.
- **Motion**: none.

### Agent List

- **Structure**: one or two lines per row, with `>` marking the selected/detail row.
- **Variants**: wide list, medium list, narrow compact list.
- **Spacing**: `row-main`, optional `row-detail`.
- **States**: working, idle, unknown, degraded evidence.
- **Accessibility**: row text includes client, state, safe location, and process/workspace where width allows.
- **Motion**: none.

### Detail Panel

- **Structure**: field/value lines for the selected row in wide mode.
- **Variants**: present only in wide mode with at least one row.
- **Spacing**: `pad-x`, `gap-1`.
- **States**: normal and degraded banner inherited from Shell.
- **Accessibility**: labels are textual and privacy-safe.
- **Motion**: none.

### Empty State

- **Structure**: calm message plus refresh hint and privacy note.
- **Variants**: normal empty and degraded empty.
- **Spacing**: `gap-1`.
- **States**: empty, degraded.
- **Accessibility**: no hidden color-only meaning.
- **Motion**: none.

### Help Overlay

- **Structure**: centered keyboard reference over the current dashboard.
- **Variants**: same overlay in wide, medium, and narrow terminals.
- **Spacing**: bounded centered panel.
- **States**: visible when help is toggled by `?` or `h`.
- **Accessibility**: keyboard bindings are written as text.
- **Motion**: none.

## 6. Motion & Interaction

### Timing

No animation is used in this terminal branch.

### Rules

- Existing controls remain: `q`, Esc, Ctrl-C quit; `r` and `R` refresh.
- Selection controls are read-only: `j` / Down select next, `k` / Up select previous, Home/End jump, and PageUp/PageDown move by a page-sized step.
- Help controls are read-only: `?` and `h` toggle the keyboard overlay.
- Do not add mutating actions, search, pinning, grouping, or previews.
- The active row marker follows selection in this branch.

## 7. Depth & Surface

### Strategy

Use borders plus tonal emphasis. Ratatui panels provide structure; semantic color and bold/dim modifiers provide depth.

| Level | Treatment | Usage |
|-------|-----------|-------|
| Base | default background | Full terminal area |
| Panel | bordered block | Main list and wide detail panel |
| Active | cyan marker and bold text | Selected row/detail target |
| Warning | red status text | Degraded refresh state |

### Scope Exclusions

- No non-loopback networking; the shipped daemon API is loopback-only.
- No previews from terminal content.
- No mutating actions beyond existing quit and refresh.
- No search, pinning, grouping, Git/PR enrichment, persistence, or terminal-content scraping.
- No external dependencies.
- No unsafe code.
- No raw `session_name` or `window_name` rendering; safe location format is `unknown:<window_index> unknown` plus pane id when needed.
