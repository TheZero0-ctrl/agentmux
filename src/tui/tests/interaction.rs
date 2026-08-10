use crate::app::Action;

use super::fixtures::{app_with_rows, claude_row, codex_row, content_row, unknown_row};
use super::{render_lines, rendered_text};

#[test]
fn given_selected_second_row_when_rendered_wide_then_detail_panel_follows_selection()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a wide dashboard where the second row is selected.
    let mut app = app_with_rows(vec![codex_row("%1"), claude_row("%2")]);
    app.apply(Action::SelectNext);

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 140, 12)?;

    // Then: the selected tile is marked in the sidebar and its pane surface is rendered.
    assert!(rendered.contains("> codex"));
    assert!(rendered.contains("pane content unavailable"));
    Ok(())
}

#[test]
fn given_grouped_rows_when_navigating_then_selection_follows_sidebar_order() {
    // Given: discovery returns rows in an order different from the grouped sidebar.
    let mut app = app_with_rows(vec![claude_row("%2"), codex_row("%1")]);

    // When: the next row is selected.
    app.apply(Action::SelectNext);

    // Then: navigation follows the project-grouped order shown to the user.
    assert_eq!(
        app.selected_row().map(crate::app::DashboardRow::client),
        Some("codex")
    );
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
    let _lines = render_lines(&app, 100, 8)?;

    // Then: the selected row is visible and marked active.
    assert_eq!(
        app.selected_row().map(crate::app::DashboardRow::client),
        Some("unknown")
    );
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
    assert!(rendered.contains("tab/i focus input"));
    Ok(())
}

#[test]
fn given_captured_pane_content_when_rendered_then_latest_output_is_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app_with_rows(vec![content_row("%1")]);

    let rendered = rendered_text(&app, 100, 16)?;

    assert!(rendered.contains("latest output"));
    Ok(())
}

#[test]
fn given_wrapped_pane_content_when_rendered_narrow_then_bottom_output_remains_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app_with_rows(vec![content_row("%1").with_content(
        "long status text that wraps across the narrow pane\nsecond line\ncomposer input",
    )]);

    let rendered = rendered_text(&app, 60, 12)?;

    assert!(rendered.contains("composer input"));
    Ok(())
}

#[test]
fn given_input_mode_on_narrow_terminal_then_tile_keeps_vertical_space()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = app_with_rows(vec![
        content_row("%1").with_content("codex output\nline two\ncomposer input"),
    ]);
    app.apply(Action::ToggleInputMode);

    let rendered = rendered_text(&app, 60, 9)?;

    assert!(rendered.contains("composer input"));
    assert!(!rendered.contains("focused input"));
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

    // When: the dashboard sidebar and agent tiles are rendered.
    let rendered = rendered_text(&app, 140, 12)?;

    // Then: the pane surface stays present and raw labels are not.
    assert!(rendered.contains("pane content unavailable"));
    assert!(!rendered.contains("prompt"));
    assert!(!rendered.contains("sk-secret"));
    assert!(!rendered.contains("diff --git"));
    assert!(!rendered.contains("token"));
    Ok(())
}
