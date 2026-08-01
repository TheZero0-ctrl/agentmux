use super::AgentProjectionRow;

pub(super) fn is_visible_agent_row(row: &AgentProjectionRow) -> bool {
    row.client() != "unknown" || is_authoritative_source(row.evidence_source())
}

fn is_authoritative_source(source: &str) -> bool {
    matches!(source, "hook" | "marker" | "structured_log")
}
