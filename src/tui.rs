//! Ratatui rendering for the agentmux dashboard.

use ratatui::Frame;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::app::{App, DashboardRow};

#[cfg(test)]
mod tests;

/// Render the live local agent discovery dashboard.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let width = usize::from(frame.area().width);
    let layout = DashboardLayout::for_width(width);
    frame.render_widget(Paragraph::new(lines_for(app, layout, width)), frame.area());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardLayout {
    Wide,
    Medium,
    Narrow,
}

impl DashboardLayout {
    const fn for_width(width: usize) -> Self {
        if width >= 137 {
            Self::Wide
        } else if width >= 80 {
            Self::Medium
        } else {
            Self::Narrow
        }
    }
}

fn lines_for(app: &App, layout: DashboardLayout, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from("agentmux"),
        Line::from("Controls: q quit | Esc quit | Ctrl-C quit | r refresh"),
        Line::from(status_line(app, app.rows().len())),
    ];

    if app.rows().is_empty() {
        lines.push(Line::from("Empty: no agents discovered yet"));
        lines.push(Line::from("Unknown: unavailable fields display as unknown"));
        return lines;
    }

    lines.push(Line::from(header_for(layout, width)));
    lines.extend(
        app.rows()
            .iter()
            .map(|row| Line::from(row_line(row, layout, width))),
    );
    lines
}

fn status_line(app: &App, row_count: usize) -> String {
    app.degraded_message().map_or_else(
        || format!("Status: live projection rows: {row_count}"),
        |message| format!("Status: degraded - {message}; showing last good rows"),
    )
}

fn header_for(layout: DashboardLayout, width: usize) -> String {
    match layout {
        DashboardLayout::Wide => join_cells(
            [
                cell("CLIENT/CONF", 16),
                cell("STATE", 18),
                cell("SESSION/WINDOW", 22),
                cell("PANE", 7),
                cell("PID/PROCESS", 20),
                cell("WORKSPACE", 24),
                cell("EVIDENCE", 24),
            ]
            .as_ref(),
        ),
        DashboardLayout::Medium => join_cells(
            [
                cell("CLIENT", 16),
                cell("STATE", 14),
                cell("PROCESS", 18),
                cell("WORKSPACE", 20),
                cell("LOCATION", width.saturating_sub(72)),
            ]
            .as_ref(),
        ),
        DashboardLayout::Narrow => join_cells(
            [
                cell("CLIENT", 14),
                cell("STATE", 12),
                cell("LOCATION", width.saturating_sub(28)),
            ]
            .as_ref(),
        ),
    }
}

fn row_line(row: &DashboardRow, layout: DashboardLayout, width: usize) -> String {
    match layout {
        DashboardLayout::Wide => join_cells(
            [
                cell(&client_with_confidence(row), 16),
                cell(row.state(), 18),
                cell(&session_window(row), 22),
                cell(row.pane_id(), 7),
                cell(&pid_process(row), 20),
                cell(row.workspace(), 24),
                cell(&evidence(row), 24),
            ]
            .as_ref(),
        ),
        DashboardLayout::Medium => join_cells(
            [
                cell(row.client(), 16),
                cell(row.state(), 14),
                cell(&pid_process(row), 18),
                cell(row.workspace(), 20),
                cell(&location(row), width.saturating_sub(72)),
            ]
            .as_ref(),
        ),
        DashboardLayout::Narrow => join_cells(
            [
                cell(row.client(), 14),
                cell(row.state(), 12),
                cell(&location(row), width.saturating_sub(28)),
            ]
            .as_ref(),
        ),
    }
}

fn client_with_confidence(row: &DashboardRow) -> String {
    format!("{}/{}", row.client(), row.client_confidence())
}

fn session_window(row: &DashboardRow) -> String {
    format!("unknown:{} unknown", row.window_index())
}

fn pid_process(row: &DashboardRow) -> String {
    format!("{} {}", row.pid(), row.process_name())
}

fn location(row: &DashboardRow) -> String {
    format!("{} {}", session_window(row), row.pane_id())
}

fn evidence(row: &DashboardRow) -> String {
    format!(
        "{} {} {}",
        row.evidence_source(),
        row.evidence_freshness(),
        row.evidence_confidence()
    )
}

fn join_cells(cells: &[String]) -> String {
    cells.join(" ")
}

fn cell(value: &str, width: usize) -> String {
    format!("{:<width$}", truncate(value, width), width = width)
}

fn truncate(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut chars = value.chars();
    let mut output = String::new();

    for _ in 0..width {
        match chars.next() {
            Some(character) => output.push(character),
            None => return output,
        }
    }

    if chars.next().is_some() {
        output.pop();
        output.push('~');
    }

    output
}
