//! Ratatui rendering for the agentmux dashboard.

use std::collections::BTreeMap;

use ansi_to_tui::IntoText;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Text;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, DashboardRow};

use self::theme::{
    border_style, focus_border_style, muted_style, state_style, status_style, title_style,
};

mod help;
#[cfg(test)]
mod tests;
mod theme;

const WIDE_WIDTH: usize = 137;
const MEDIUM_WIDTH: usize = 80;

/// Render the live local agent discovery dashboard.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let layout = DashboardLayout::for_width(usize::from(area.width));
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if matches!(layout, DashboardLayout::Narrow) {
                2
            } else {
                3
            }),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let [header, main, footer] = shell.as_ref() else {
        return;
    };

    render_header(frame, *header, app);
    render_main(frame, *main, app, layout);
    render_footer(frame, *footer, layout, app);
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
    if app.is_sidebar_visible() {
        let sidebar_width = if area.width < 80 { 22 } else { 28 };
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(1)])
            .split(area);
        let [sidebar, main] = chunks.as_ref() else {
            return;
        };
        render_sidebar(frame, *sidebar, app);
        render_workspace(frame, *main, app, layout);
    } else {
        render_workspace(frame, area, app, layout);
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, layout: DashboardLayout, app: &App) {
    if app.is_input_mode() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "INPUT: keys go to focused pane | Esc exit input",
                focus_border_style(),
            ))),
            area,
        );
        return;
    }
    let footer = match layout {
        DashboardLayout::Narrow => "j/k select | tab/i input | enter/o switch | s sidebar | q quit",
        DashboardLayout::Medium | DashboardLayout::Wide => {
            "j/k select | tab/i input | enter/o switch | s sidebar | r refresh | q quit"
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer, muted_style()))),
        area,
    );
}

fn render_workspace(frame: &mut Frame<'_>, area: Rect, app: &App, layout: DashboardLayout) {
    if app.is_input_mode() && !matches!(layout, DashboardLayout::Narrow) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(area);
        if let [banner, tiles] = chunks.as_ref() {
            let banner_block = Block::default()
                .title(" focused input ")
                .borders(Borders::ALL)
                .border_style(focus_border_style());
            frame.render_widget(
                Paragraph::new("Keyboard input is forwarded to the focused agent pane")
                    .block(banner_block),
                *banner,
            );
            render_tiles(frame, *tiles, app);
        }
    } else {
        render_tiles(frame, area, app);
    }
}

fn render_sidebar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .title(" sidebar: agents ")
        .borders(Borders::ALL)
        .border_style(border_style());
    let lines = sidebar_lines(app, usize::from(area.height.saturating_sub(2)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_tiles(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(agent) = app.selected_row() else {
        let block = Block::default()
            .title(" agent preview ")
            .borders(Borders::ALL)
            .border_style(border_style());
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("No selected agent preview", muted_style())),
                Line::from("Select an agent in the sidebar to preview it"),
            ])
            .block(block),
            area,
        );
        return;
    };
    render_agent_tile(frame, area, app, agent);
}

fn render_agent_tile(frame: &mut Frame<'_>, area: Rect, app: &App, agent: &DashboardRow) {
    let active = app
        .selected_row()
        .is_some_and(|selected| selected.agent_id() == agent.agent_id());
    let focus = if active && app.is_input_mode() {
        " INPUT >"
    } else if active {
        " >"
    } else {
        ""
    };
    let title = format!("{} {} ", focus, agent.client());
    let border = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if active {
            focus_border_style()
        } else {
            border_style()
        });
    let text = tile_text(agent);
    let content_width = area.width.saturating_sub(2);
    let visible_height = area.height.saturating_sub(2);
    let terminal_snapshot = agent.content().contains('\u{1b}');
    let rendered_lines = if terminal_snapshot {
        text.lines.len()
    } else {
        wrapped_line_count(&text, content_width)
    };
    let paragraph = Paragraph::new(text);
    let paragraph = if terminal_snapshot {
        paragraph
    } else {
        paragraph.wrap(Wrap { trim: false })
    };
    let scroll = u16::try_from(rendered_lines.saturating_sub(usize::from(visible_height)))
        .unwrap_or(u16::MAX);
    frame.render_widget(paragraph.scroll((scroll, 0)).block(border), area);
}

fn wrapped_line_count(text: &Text<'_>, width: u16) -> usize {
    let width = usize::from(width);
    if width == 0 {
        return 0;
    }
    text.lines
        .iter()
        .map(|line| line.width().max(1).div_ceil(width))
        .sum()
}

fn sidebar_lines(app: &App, capacity: usize) -> Vec<Line<'static>> {
    if app.rows().is_empty() {
        return empty_lines(app);
    }
    let selected = app.selected_index().unwrap_or_default();
    let mut grouped = BTreeMap::<&str, Vec<(usize, &DashboardRow)>>::new();
    for (index, row) in app.rows().iter().enumerate() {
        grouped
            .entry(row.workspace())
            .or_default()
            .push((index, row));
    }

    let mut lines = Vec::new();
    let mut selected_line = 0;
    for (workspace, rows) in grouped {
        lines.push(Line::from(Span::styled(
            format!("▾ {workspace}"),
            muted_style(),
        )));
        for (index, row) in rows {
            if index == selected {
                selected_line = lines.len();
            }
            let marker = if index == selected { ">" } else { " " };
            lines.push(
                Line::from(vec![
                    Span::raw(format!("{marker} ")),
                    Span::styled(row.client().to_owned(), state_style(row.state())),
                    Span::raw(format!(" {}", row.state())),
                ])
                .style(if marker == ">" {
                    focus_border_style()
                } else {
                    border_style()
                }),
            );
        }
    }
    let start = selected_line.saturating_sub(capacity.saturating_sub(1));
    lines.into_iter().skip(start).take(capacity).collect()
}

fn tile_text(row: &DashboardRow) -> Text<'static> {
    if let Some(text) = row.rendered_content() {
        return text.clone();
    }
    let content = if row.content().is_empty() {
        "pane content unavailable"
    } else {
        row.content()
    };
    match content.as_bytes().to_vec().into_text() {
        Ok(text) => text,
        Err(_error) => Text::from(content.to_owned()),
    }
}

fn header_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("agentmux", title_style())),
        Line::from(status_text(app)).style(status_style(app)),
        Line::from(Span::styled("local tmux/procfs discovery", muted_style())),
    ]
}

fn status_text(app: &App) -> String {
    if app.is_input_mode() {
        "INPUT MODE - focused pane receives keyboard input".to_owned()
    } else {
        app.degraded_message().map_or_else(
            || {
                format!(
                    "live projection rows: {} | shown panes: {}",
                    app.rows().len(),
                    usize::from(app.selected_row().is_some())
                )
            },
            |message| format!("degraded - {message}; showing last good rows"),
        )
    }
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
