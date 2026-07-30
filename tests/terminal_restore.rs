//! Test-only Ratatui integration coverage for terminal restoration.
//!
//! This test intentionally runs against a real terminal, draws minimal content,
//! returns an I/O error from the closure, and asserts that the error surfaces.

use std::io;

#[test]
#[ignore = "requires a real terminal"]
fn given_ratatui_run_when_closure_errors_then_the_error_is_returned() {
    let result: io::Result<()> = ratatui::run(|terminal| {
        terminal.draw(|frame| {
            frame.render_widget(ratatui::widgets::Paragraph::new("ok"), frame.area());
        })?;

        Err(io::Error::other("terminal_restore"))
    });

    assert!(result.is_err());
}
