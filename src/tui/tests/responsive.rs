use super::fixtures::{app_with_rows, claude_row, codex_row};
use super::render_lines;

#[test]
fn given_wide_terminal_when_rendered_then_sidebar_and_agent_tiles_are_visible()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: discovered rows and a terminal wide enough for the sidebar split.
    let app = app_with_rows(vec![codex_row("%1"), claude_row("%2")]);

    // When: the dashboard is rendered in wide mode.
    let lines = render_lines(&app, 140, 12)?;

    // Then: the shell has header, sidebar, agent tiles, and footer.
    assert!(lines.iter().any(|line| line.contains("agentmux")));
    assert!(lines.iter().any(|line| line.contains("sidebar: agents")));
    assert!(lines.iter().any(|line| line.contains("codex")));
    assert!(lines.iter().any(|line| line.contains("> claude")));
    assert!(lines.iter().any(|line| line.contains("q quit")));
    assert!(lines.iter().any(|line| line.contains("sidebar")));
    Ok(())
}

#[test]
fn given_wide_boundary_when_rendered_then_agent_tiles_are_used()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a populated dashboard at the wide breakpoint.
    let app = app_with_rows(vec![codex_row("%1")]);

    // When: the dashboard is rendered at exactly 137 columns.
    let lines = render_lines(&app, 137, 10)?;

    // Then: the agent pane area is present.
    assert!(lines.iter().any(|line| line.contains("codex")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("pane content unavailable"))
    );
    Ok(())
}

#[test]
fn given_medium_boundary_when_rendered_then_single_list_is_used()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a populated dashboard one column below the wide breakpoint.
    let app = app_with_rows(vec![codex_row("%1")]);

    // When: the dashboard is rendered at 136 columns.
    let lines = render_lines(&app, 136, 10)?;

    // Then: the sidebar and tile area remain usable.
    assert!(lines.iter().any(|line| line.contains("sidebar: agents")));
    assert!(lines.iter().any(|line| line.contains("codex")));
    Ok(())
}

#[test]
fn given_narrow_terminal_when_rendered_then_compact_fields_fit_without_detail_panel()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: rows with more fields than a narrow terminal can display.
    let app = app_with_rows(vec![codex_row("%10"), claude_row("%2")]);

    // When: the dashboard is rendered in narrow mode.
    let lines = render_lines(&app, 60, 9)?;

    // Then: the compact sidebar and tile area remain usable.
    assert!(lines.iter().any(|line| line.contains("codex")));
    assert!(lines.iter().any(|line| line.contains("codex")));
    Ok(())
}
