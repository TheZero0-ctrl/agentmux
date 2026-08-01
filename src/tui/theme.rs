use ratatui::style::{Color, Modifier, Style};

use crate::app::{App, DashboardRow};

pub(super) fn title_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn border_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(super) fn muted_style() -> Style {
    Style::default().fg(Color::Gray)
}

pub(super) fn status_style(app: &App) -> Style {
    if app.degraded_message().is_some() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    }
}

pub(super) fn marker_style(active: bool, row: &DashboardRow) -> Style {
    if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        state_style(row.state())
    }
}

pub(super) fn state_style(state: &str) -> Style {
    match state {
        "working" => Style::default().fg(Color::Yellow),
        "idle" => Style::default().fg(Color::Green),
        "unknown" => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::Gray),
    }
}
