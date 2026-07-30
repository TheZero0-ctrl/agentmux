//! Ratatui rendering for the agentmux dashboard.

use ratatui::Frame;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::app::App;

/// Render the dashboard shell.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let _ = app;
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("agentmux"),
            Line::from("No agents detected yet"),
            Line::from("q quit"),
        ]),
        frame.area(),
    );
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::App;

    use super::render;

    #[test]
    fn given_empty_dashboard_when_rendered_then_it_shows_the_expected_shell()
    -> Result<(), Box<dyn Error>> {
        // Given: the dashboard app state and a test terminal backend.
        let backend = TestBackend::new(24, 3);
        let mut terminal = Terminal::new(backend)?;
        let app = App::new();

        // When: the TUI render seam is exercised.
        // Then: the expected dashboard shell should be present.
        terminal.draw(|frame| render(frame, &app))?;

        terminal.backend().assert_buffer_lines([
            "agentmux                ",
            "No agents detected yet  ",
            "q quit                  ",
        ]);

        Ok(())
    }
}
