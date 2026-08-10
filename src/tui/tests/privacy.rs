use super::fixtures::{app_with_rows, codex_row};
use super::rendered_text;

#[test]
fn given_sensitive_session_window_labels_when_rendered_then_raw_metadata_is_absent()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a row that would leak if raw tmux session/window labels reached rendering.
    let app = app_with_rows(vec![codex_row("%1").with_location(
        "prompt sk-live-secret",
        "0",
        "diff --git raw stderr",
    )]);

    // When: the dashboard is rendered.
    let rendered = rendered_text(&app, 140, 12)?;

    // Then: forbidden fixture strings are absent from the pane surface.
    assert!(!rendered.contains("prompt"));
    assert!(!rendered.contains("sk-live-secret"));
    assert!(!rendered.contains("diff --git"));
    assert!(!rendered.contains("raw stderr"));
    assert!(rendered.contains("pane content unavailable"));
    Ok(())
}
