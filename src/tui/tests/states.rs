use ratatui::style::{Color, Modifier};

use super::fixtures::{app_with_rows, codex_row, unknown_row};
use super::{render_lines, rendered_text};
use crate::app::App;

#[test]
fn given_empty_dashboard_when_rendered_then_empty_state_is_explicit()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: no discovered dashboard rows.
    let app = App::new();

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 100, 8)?;

    // Then: the empty state and refresh hint are visible.
    assert!(rendered.contains("No agents found"));
    assert!(rendered.contains("press r to refresh"));
    assert!(rendered.contains("labels hidden for privacy"));
    Ok(())
}

#[test]
fn given_empty_wide_dashboard_when_rendered_then_detail_panel_is_absent()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: no discovered dashboard rows and a wide terminal.
    let app = App::new();

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 140, 8)?;

    // Then: the empty list uses the main area without an empty detail panel.
    assert!(rendered.contains("No agents found"));
    assert!(!rendered.contains("details"));
    Ok(())
}

#[test]
fn given_degraded_refresh_when_rendered_then_banner_retains_last_good_rows()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: refresh failed after a previously successful row set.
    let mut app = app_with_rows(vec![codex_row("%1")]);
    app.mark_degraded();

    // When: the dashboard is rendered.
    let lines = render_lines(&app, 100, 10)?;

    // Then: the degraded status is explicit and the row still renders.
    assert!(lines.iter().any(|line| line.contains("degraded")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("showing last good rows"))
    );
    assert!(lines.iter().any(|line| line.contains("codex")));
    Ok(())
}

#[test]
fn given_state_labels_when_rendered_then_semantic_styles_are_distinct()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: working and unknown rows rendered together.
    let app = app_with_rows(vec![codex_row("%1"), unknown_row("%2")]);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 12))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| super::render(frame, &app))?;
    let rendered = terminal.backend().buffer();

    // Then: semantic styles distinguish active and unknown row markers.
    let active_cell = rendered.cell((2, 4));
    let unknown_cell = rendered.cell((2, 6));
    assert_eq!(active_cell.map(|cell| cell.fg), Some(Color::Cyan));
    assert_eq!(active_cell.map(|cell| cell.modifier), Some(Modifier::BOLD));
    assert_eq!(unknown_cell.map(|cell| cell.fg), Some(Color::DarkGray));
    Ok(())
}

#[test]
fn given_zero_width_when_truncated_then_empty_string_is_returned() {
    // Given: non-empty text and a zero-width cell.
    // When: truncation runs for that cell.
    // Then: no replacement marker is emitted into an impossible width.
    assert_eq!(super::super::truncate("agent", 0), "");
}
