use std::error::Error;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::{App, DashboardRow};

use super::{render, truncate};

#[test]
fn given_populated_rows_when_rendered_wide_then_shared_columns_are_visible_in_sorted_order()
-> Result<(), Box<dyn Error>> {
    // Given: rows arrive already ordered by the shared projection.
    let app = app_with_rows(vec![claude_row("%2"), codex_row("%10")]);
    let mut terminal = Terminal::new(TestBackend::new(140, 7))?;

    // When: the wide dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the full shared projection columns are readable in the provided order.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 2",
            "CLIENT/CONF      STATE              SESSION/WINDOW         PANE    PID/PROCESS          WORKSPACE                EVIDENCE                ",
            "claude/low       idle               unknown:2 unknown      %2      202 claude           ops                      process fresh low       ",
            "codex/low        working            unknown:3 unknown      %10     101 codex            project                  process fresh low       ",
            "",
        ],
        140,
    ));
    Ok(())
}

#[test]
fn given_width_below_wide_fixed_row_when_rendered_then_medium_layout_is_used()
-> Result<(), Box<dyn Error>> {
    // Given: a populated row and a terminal one column too narrow for the fixed wide row.
    let app = app_with_rows(vec![codex_row("%1")]);
    let mut terminal = Terminal::new(TestBackend::new(136, 5))?;

    // When: the dashboard is rendered at the wide boundary minus one.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the medium layout is used so the fixed wide row cannot overflow.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 1",
            "CLIENT           STATE          PROCESS            WORKSPACE            LOCATION                                                       ",
            "codex            working        101 codex          project              unknown:3 unknown %1                                            ",
        ],
        136,
    ));
    Ok(())
}

#[test]
fn given_width_equal_to_wide_fixed_row_when_rendered_then_wide_layout_is_used()
-> Result<(), Box<dyn Error>> {
    // Given: a populated row and a terminal exactly wide enough for all fixed wide cells.
    let app = app_with_rows(vec![codex_row("%1")]);
    let mut terminal = Terminal::new(TestBackend::new(137, 5))?;

    // When: the dashboard is rendered at the wide boundary.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the wide layout is used without truncating fixed columns.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 1",
            "CLIENT/CONF      STATE              SESSION/WINDOW         PANE    PID/PROCESS          WORKSPACE                EVIDENCE                ",
            "codex/low        working            unknown:3 unknown      %1      101 codex            project                  process fresh low       ",
        ],
        137,
    ));
    Ok(())
}

#[test]
fn given_populated_rows_when_rendered_medium_then_location_is_combined()
-> Result<(), Box<dyn Error>> {
    // Given: two projected dashboard rows.
    let app = app_with_rows(vec![claude_row("%2"), codex_row("%10")]);
    let mut terminal = Terminal::new(TestBackend::new(100, 7))?;

    // When: the medium dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: location combines session, window, and pane while core columns remain.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 2",
            "CLIENT           STATE          PROCESS            WORKSPACE            LOCATION                    ",
            "claude           idle           202 claude         ops                  unknown:2 unknown %2       ",
            "codex            working        101 codex          project              unknown:3 unknown %10      ",
            "",
        ],
        100,
    ));
    Ok(())
}

#[test]
fn given_populated_rows_when_rendered_narrow_then_only_client_state_location_remain()
-> Result<(), Box<dyn Error>> {
    // Given: projected rows contain more fields than the narrow layout can show.
    let app = app_with_rows(vec![claude_row("%2"), codex_row("%10")]);
    let mut terminal = Terminal::new(TestBackend::new(60, 7))?;

    // When: the narrow dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: only client, state, and deterministic location text remain.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 2",
            "CLIENT         STATE        LOCATION                        ",
            "claude         idle         unknown:2 unknown %2            ",
            "codex          working      unknown:3 unknown %10           ",
            "",
        ],
        60,
    ));
    Ok(())
}

#[test]
fn given_empty_dashboard_when_rendered_then_empty_and_unknown_state_text_is_explicit()
-> Result<(), Box<dyn Error>> {
    // Given: no discovered dashboard rows.
    let app = App::new();
    let mut terminal = Terminal::new(TestBackend::new(80, 5))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the empty state and unknown-field convention are visible without color.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 0",
            "Empty: no agents discovered yet",
            "Unknown: unavailable fields display as unknown",
        ],
        80,
    ));
    Ok(())
}

#[test]
fn given_unknown_fields_when_rendered_then_unknown_is_visible_in_row_cells()
-> Result<(), Box<dyn Error>> {
    // Given: process and client projection fields are unavailable.
    let app = app_with_rows(vec![unknown_row("%1")]);
    let mut terminal = Terminal::new(TestBackend::new(140, 6))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: unknown is printed as text, not implied by color.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: live projection rows: 1",
            "CLIENT/CONF      STATE              SESSION/WINDOW         PANE    PID/PROCESS          WORKSPACE                EVIDENCE                ",
            "unknown/unknown  unknown            unknown:0 unknown      %1      unknown unknown      unknown                  tmux stale low          ",
            "",
        ],
        140,
    ));
    Ok(())
}

#[test]
fn given_degraded_refresh_when_rendered_then_banner_retains_last_good_rows()
-> Result<(), Box<dyn Error>> {
    // Given: refresh failed after a previously successful row set.
    let mut app = app_with_rows(vec![codex_row("%1")]);
    app.mark_degraded();
    let mut terminal = Terminal::new(TestBackend::new(100, 6))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the degraded status is explicit and the row still renders.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
            "Status: degraded - dashboard refresh degraded; showing last good rows",
            "CLIENT           STATE          PROCESS            WORKSPACE            LOCATION                    ",
            "codex            working        101 codex          project              unknown:3 unknown %1       ",
            "",
        ],
        100,
    ));
    Ok(())
}

#[test]
fn given_tiny_height_when_rendered_then_title_and_refresh_control_are_not_malformed()
-> Result<(), Box<dyn Error>> {
    // Given: a terminal too short for the table.
    let app = app_with_rows(vec![codex_row("%1")]);
    let mut terminal = Terminal::new(TestBackend::new(60, 2))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: clipping remains deterministic and keeps the controls line intact.
    terminal.backend().assert_buffer_lines(pad_lines(
        [
            "agentmux",
            "Controls: q quit | Esc quit | Ctrl-C quit | r refresh",
        ],
        60,
    ));
    Ok(())
}

#[test]
fn given_sensitive_session_window_labels_when_rendered_then_raw_metadata_is_absent()
-> Result<(), Box<dyn Error>> {
    // Given: a row that would leak if raw tmux session/window labels reached rendering.
    let app = app_with_rows(vec![codex_row("%1").with_location(
        "prompt sk-live-secret",
        "0",
        "diff --git raw stderr",
    )]);
    let mut terminal = Terminal::new(TestBackend::new(100, 6))?;

    // When: the dashboard is rendered.
    terminal.draw(|frame| render(frame, &app))?;

    // Then: the rendered terminal buffer never contains forbidden fixture strings.
    let rendered = terminal.backend().to_string();
    assert!(!rendered.contains("prompt"));
    assert!(!rendered.contains("sk-live-secret"));
    assert!(!rendered.contains("diff --git"));
    assert!(!rendered.contains("raw stderr"));
    assert!(rendered.contains("unknown:0 unknown %1"));
    assert!(!rendered.contains("/home/alice"));
    assert!(!rendered.contains("token="));
    Ok(())
}

#[test]
fn given_zero_width_when_truncated_then_empty_string_is_returned() {
    // Given: non-empty text and a zero-width cell.
    // When: truncation runs for that cell.
    // Then: no replacement marker is emitted into an impossible width.
    assert_eq!(truncate("agent", 0), "");
}

fn app_with_rows(rows: Vec<DashboardRow>) -> App {
    let mut app = App::new();
    app.replace_rows(rows);
    app
}

fn codex_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%1")
        .with_pane(pane_id)
        .with_location("work", "3", "editor")
        .with_process("101", "codex")
        .with_client("codex", "low")
        .with_workspace("project")
        .with_state("working")
        .with_evidence("process", "fresh", "low")
}

fn claude_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%2")
        .with_pane(pane_id)
        .with_location("qa", "2", "logs")
        .with_process("202", "claude")
        .with_client("claude", "low")
        .with_workspace("ops")
        .with_state("idle")
        .with_evidence("process", "fresh", "low")
}

fn unknown_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%1")
        .with_pane(pane_id)
        .with_location("unknown", "0", "unknown")
        .with_process("unknown", "unknown")
        .with_client("unknown", "unknown")
        .with_workspace("unknown")
        .with_state("unknown")
        .with_evidence("tmux", "stale", "low")
}

fn pad_lines<const N: usize>(lines: [&str; N], width: usize) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| format!("{line:<width$}"))
        .collect()
}
