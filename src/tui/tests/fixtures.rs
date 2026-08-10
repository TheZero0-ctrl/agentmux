use crate::app::{App, DashboardRow};

pub(super) fn app_with_rows(rows: Vec<DashboardRow>) -> App {
    let mut app = App::new();
    app.replace_rows(rows);
    app
}

pub(super) fn codex_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%1")
        .with_pane(pane_id)
        .with_location("work", "3", "editor")
        .with_process("101", "codex")
        .with_client("codex", "low")
        .with_workspace("project")
        .with_state("working")
        .with_evidence("process", "fresh", "low")
}

pub(super) fn content_row(pane_id: &str) -> DashboardRow {
    codex_row(pane_id).with_content("line one\nline two\nlatest output")
}

pub(super) fn claude_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%2")
        .with_pane(pane_id)
        .with_location("qa", "2", "logs")
        .with_process("202", "claude")
        .with_client("claude", "low")
        .with_workspace("ops")
        .with_state("idle")
        .with_evidence("process", "fresh", "low")
}

pub(super) fn unknown_row(pane_id: &str) -> DashboardRow {
    DashboardRow::for_test("pane:%1")
        .with_pane(pane_id)
        .with_location("unknown", "0", "unknown")
        .with_process("unknown", "unknown")
        .with_client("unknown", "unknown")
        .with_workspace("unknown")
        .with_state("unknown")
        .with_evidence("tmux", "stale", "low")
}
