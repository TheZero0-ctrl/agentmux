use std::error::Error;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::render;

mod fixtures;
mod interaction;
mod privacy;
mod responsive;
mod states;

fn render_lines(
    app: &crate::app::App,
    width: u16,
    height: u16,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;

    terminal.draw(|frame| render(frame, app))?;

    Ok(terminal
        .backend()
        .to_string()
        .lines()
        .map(str::trim_end)
        .map(str::to_owned)
        .collect())
}

fn rendered_text(app: &crate::app::App, width: u16, height: u16) -> Result<String, Box<dyn Error>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;

    terminal.draw(|frame| render(frame, app))?;

    Ok(terminal.backend().to_string())
}
