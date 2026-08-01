use crate::app::Action;

use super::fixtures::{app_with_rows, claude_row, codex_row, unknown_row};
use super::{render_lines, rendered_text};

#[test]
fn given_selected_second_row_when_rendered_wide_then_detail_panel_follows_selection()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a wide dashboard where the second row is selected.
    let mut app = app_with_rows(vec![codex_row("%1"), claude_row("%2")]);
    app.apply(Action::SelectNext);

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 140, 12)?;

    // Then: the detail panel shows the selected row, not the first row.
    assert!(rendered.contains("client: claude"));
    assert!(rendered.contains("pane: %2"));
    assert!(!rendered.contains("client: codex"));
    Ok(())
}

#[test]
fn given_selection_below_visible_window_when_rendered_then_viewport_follows_selection()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: more rows than a short list can show and the last row selected.
    let mut app = app_with_rows(vec![
        codex_row("%1"),
        claude_row("%2"),
        unknown_row("%3"),
        codex_row("%4"),
    ]);
    app.apply(Action::SelectLast);

    // When: the dashboard is rendered with a short main area.
    let lines = render_lines(&app, 100, 8)?;

    // Then: the selected row is visible and marked active.
    assert!(lines.iter().any(|line| line.contains("> codex")));
    assert!(lines.iter().any(|line| line.contains("%4")));
    Ok(())
}

#[test]
fn given_help_visible_when_rendered_then_help_overlay_is_shown()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a dashboard with help toggled open.
    let mut app = app_with_rows(vec![codex_row("%1")]);
    app.apply(Action::ToggleHelp);

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 100, 12)?;

    // Then: static privacy-safe keyboard help is visible.
    assert!(rendered.contains("help"));
    assert!(rendered.contains("j/down"));
    assert!(rendered.contains("k/up"));
    assert!(rendered.contains("labels hidden for privacy"));
    Ok(())
}

#[test]
fn given_sensitive_selected_row_when_rendered_wide_then_detail_panel_stays_private()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a selected row with sensitive raw tmux labels behind the presentation row.
    let mut app = app_with_rows(vec![
        codex_row("%1"),
        claude_row("%2").with_location("prompt sk-secret", "7", "diff --git token"),
    ]);
    app.apply(Action::SelectNext);

    // When: the wide dashboard detail panel is rendered.
    let rendered = rendered_text(&app, 140, 12)?;

    // Then: safe selected details are visible and raw labels are not.
    assert!(rendered.contains("unknown:7 unknown %2"));
    assert!(!rendered.contains("prompt"));
    assert!(!rendered.contains("sk-secret"));
    assert!(!rendered.contains("diff --git"));
    assert!(!rendered.contains("token"));
    Ok(())
}
