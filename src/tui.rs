//! Ratatui rendering for the agentmux dashboard.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, DashboardRow};

use self::theme::{
    border_style, marker_style, muted_style, state_style, status_style, title_style,
};

mod help;
#[cfg(test)]
mod tests;
mod theme;
mod viewport;

const WIDE_WIDTH: usize = 137;
const MEDIUM_WIDTH: usize = 80;

/// Render the live local agent discovery dashboard.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let layout = DashboardLayout::for_width(usize::from(area.width));
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let [header, main, footer] = shell.as_ref() else {
        return;
    };

    render_header(frame, *header, app);
    render_main(frame, *main, app, layout);
    render_footer(frame, *footer, layout);
    if app.is_help_visible() {
        help::render(frame, area);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardLayout {
    Wide,
    Medium,
    Narrow,
}

impl DashboardLayout {
    const fn for_width(width: usize) -> Self {
        if width >= WIDE_WIDTH {
            Self::Wide
        } else if width >= MEDIUM_WIDTH {
            Self::Medium
        } else {
            Self::Narrow
        }
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Paragraph::new(header_lines(app)), area);
}

fn render_main(frame: &mut Frame<'_>, area: Rect, app: &App, layout: DashboardLayout) {
    if matches!(layout, DashboardLayout::Wide) && !app.rows().is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);
        let [list, details] = chunks.as_ref() else {
            render_list(frame, area, app, layout);
            return;
        };
        render_list(frame, *list, app, layout);
        render_details(frame, *details, app);
    } else {
        render_list(frame, area, app, layout);
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, layout: DashboardLayout) {
    let footer = match layout {
        DashboardLayout::Narrow => "j/k select | ? help | q quit",
        DashboardLayout::Medium | DashboardLayout::Wide => {
            "j/k or arrows select | ? help | r refresh | q quit | labels hidden for privacy"
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer, muted_style()))),
        area,
    );
}

fn render_list(frame: &mut Frame<'_>, area: Rect, app: &App, layout: DashboardLayout) {
    let block = Block::default()
        .title(" local agents ")
        .borders(Borders::ALL)
        .border_style(border_style());
    let lines = if app.rows().is_empty() {
        empty_lines(app)
    } else {
        agent_lines(
            app.rows(),
            app.selected_index(),
            layout,
            usize::from(area.width.saturating_sub(2)),
            area.height.saturating_sub(2),
        )
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .title(" details ")
        .borders(Borders::ALL)
        .border_style(border_style());
    let lines = app
        .selected_row()
        .map_or_else(|| empty_lines(app), detail_lines);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn header_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("agentmux", title_style())),
        Line::from(status_text(app)).style(status_style(app)),
        Line::from(Span::styled("local tmux/procfs discovery", muted_style())),
    ]
}

fn status_text(app: &App) -> String {
    app.degraded_message().map_or_else(
        || format!("live projection rows: {}", app.rows().len()),
        |message| format!("degraded - {message}; showing last good rows"),
    )
}

fn agent_lines(
    rows: &[DashboardRow],
    selected_index: Option<usize>,
    layout: DashboardLayout,
    width: usize,
    inner_height: u16,
) -> Vec<Line<'static>> {
    let row_height = if matches!(layout, DashboardLayout::Narrow) {
        1
    } else {
        2
    };
    let capacity = viewport::visible_capacity(inner_height, row_height);
    let selected_index = selected_index.unwrap_or_default();
    let start = viewport::start_index(rows.len(), selected_index, capacity);

    rows.iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .flat_map(|(index, row)| row_lines(row, index == selected_index, layout, width))
        .collect()
}

fn row_lines(
    row: &DashboardRow,
    active: bool,
    layout: DashboardLayout,
    width: usize,
) -> Vec<Line<'static>> {
    let marker = if active { ">" } else { "|" };
    let mut lines = vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(marker, marker_style(active, row)),
        Span::raw(" "),
        Span::styled(row.client().to_owned(), state_style(row.state())),
        Span::raw(" "),
        Span::styled(row.state().to_owned(), state_style(row.state())),
        Span::raw(" "),
        Span::raw(truncate(&row_summary(row, layout), width.saturating_sub(4))),
    ])];

    if !matches!(layout, DashboardLayout::Narrow) {
        lines.push(Line::from(Span::styled(
            format!(
                "   {} | {} | {}",
                safe_location(row),
                row.workspace(),
                evidence(row)
            ),
            muted_style(),
        )));
    }
    lines
}

fn detail_lines(row: &DashboardRow) -> Vec<Line<'static>> {
    vec![
        label_value("client", row.client()),
        label_value("state", row.state()),
        label_value("process", &pid_process(row)),
        label_value("workspace", row.workspace()),
        label_value("location", &safe_location(row)),
        label_value("pane", row.pane_id()),
        label_value("evidence", &evidence(row)),
    ]
}

fn empty_lines(app: &App) -> Vec<Line<'static>> {
    let headline = if app.degraded_message().is_some() {
        "No agents found while degraded"
    } else {
        "No agents found"
    };
    vec![
        Line::from(Span::styled(headline, muted_style())),
        Line::from(Span::raw("press r to refresh")),
        Line::from(Span::styled("labels hidden for privacy", muted_style())),
    ]
}

fn label_value(label: &'static str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), muted_style()),
        Span::raw(value.to_owned()),
    ])
}

fn row_summary(row: &DashboardRow, layout: DashboardLayout) -> String {
    match layout {
        DashboardLayout::Wide => format!("{} | {}", pid_process(row), row.workspace()),
        DashboardLayout::Medium => format!("{} | {}", pid_process(row), safe_location(row)),
        DashboardLayout::Narrow => safe_location(row),
    }
}

fn safe_location(row: &DashboardRow) -> String {
    format!("unknown:{} unknown {}", row.window_index(), row.pane_id())
}

fn pid_process(row: &DashboardRow) -> String {
    format!("{} {}", row.pid(), row.process_name())
}

fn evidence(row: &DashboardRow) -> String {
    format!(
        "{} {} {}",
        row.evidence_source(),
        row.evidence_freshness(),
        row.evidence_confidence()
    )
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
