use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::theme::{border_style, muted_style, title_style};

pub(super) fn render(frame: &mut Frame<'_>, area: Rect) {
    let Some(area) = centered_rect(area) else {
        return;
    };
    let block = Block::default()
        .title(" help ")
        .borders(Borders::ALL)
        .border_style(border_style());
    let lines = vec![
        Line::from(Span::styled("keyboard", title_style())),
        Line::from("j/down select next"),
        Line::from("k/up select previous"),
        Line::from("home/end jump"),
        Line::from("page up/down move"),
        Line::from("r refresh | q quit"),
        Line::from("j/k select | tab/i focus input | s sidebar"),
        Line::from("enter/o switch to selected tmux pane (dashboard stays running)"),
        Line::from("esc stop input"),
        Line::from(Span::styled("labels hidden for privacy", muted_style())),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn centered_rect(area: Rect) -> Option<Rect> {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(area);
    let [_, middle, _] = vertical.as_ref() else {
        return None;
    };
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(*middle);
    let [_, center, _] = horizontal.as_ref() else {
        return None;
    };
    Some(*center)
}
