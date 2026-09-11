//! Dashboard discovery implementations.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::net::SocketAddr;
use std::process::Child;
use std::time::Duration;

use crate::app::DashboardRow;
use crate::daemon::autostart::{
    AutostartError, CurrentExecutableSpawner, DashboardDaemonGuard, TcpHealthProbe,
    ensure_dashboard_daemon,
};
use crate::daemon::{DiscoveryService, api::DEFAULT_DAEMON_ADDR, client::DaemonClient};
use crate::tmux::SystemTmuxCommand;
use crate::{projection, runner::DashboardDaemonMode};
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use super::{DashboardDiscovery, DashboardOptions};

const DAEMON_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_PANE_CONTENT_CHARS: usize = 32_000;
const IDLE_CONFIRMATIONS: u8 = 2;

#[derive(Debug, Default)]
struct TerminalStateTracker {
    panes: BTreeMap<String, TrackedTerminalState>,
}

#[derive(Debug, Default)]
struct TrackedTerminalState {
    stable: Option<&'static str>,
    idle_observations: u8,
}

impl TerminalStateTracker {
    const fn new() -> Self {
        Self {
            panes: BTreeMap::new(),
        }
    }

    fn observe(&mut self, agent_id: &str, observed: &'static str) -> Option<&'static str> {
        let tracked = self.panes.entry(agent_id.to_owned()).or_default();
        if observed == "idle" {
            tracked.idle_observations = tracked.idle_observations.saturating_add(1);
            if tracked.stable == Some("idle") || tracked.idle_observations >= IDLE_CONFIRMATIONS {
                tracked.stable = Some("idle");
            }
        } else {
            tracked.stable = Some(observed);
            tracked.idle_observations = 0;
        }
        tracked.stable
    }

    fn stable(&self, agent_id: &str) -> Option<&'static str> {
        self.panes.get(agent_id).and_then(|state| state.stable)
    }

    fn retain(&mut self, agent_ids: &BTreeSet<String>) {
        self.panes
            .retain(|agent_id, _state| agent_ids.contains(agent_id));
    }
}

#[derive(Debug)]
pub(super) struct ProductionDiscovery<G = DashboardDaemonGuard<Child>, L = LocalDiscovery> {
    _daemon_guard: Option<G>,
    fallback: FallbackDiscovery<DaemonStateDiscovery, L>,
    capture_content: bool,
    dashboard_binding_installed: bool,
    terminal_states: TerminalStateTracker,
}

impl ProductionDiscovery<DashboardDaemonGuard<Child>, LocalDiscovery> {
    pub(super) fn new(options: DashboardOptions) -> Self {
        let daemon = default_daemon_addr().map(|addr| DaemonClient::new(addr, DAEMON_TIMEOUT));
        let guard = match (options.daemon(), default_daemon_addr()) {
            (DashboardDaemonMode::AutoStart, Some(addr)) => {
                normalize_daemon_guard(ensure_dashboard_daemon(
                    addr,
                    &TcpHealthProbe::new(DAEMON_TIMEOUT),
                    &CurrentExecutableSpawner,
                ))
            }
            (DashboardDaemonMode::AutoStart | DashboardDaemonMode::ExistingOnly, None)
            | (DashboardDaemonMode::ExistingOnly, Some(_)) => None,
        };
        let mut discovery = Self::from_parts(daemon, guard, LocalDiscovery::new());
        discovery.capture_content = true;
        discovery.dashboard_binding_installed =
            SystemTmuxCommand.install_dashboard_binding().is_ok();
        discovery
    }
}

impl<G, L> ProductionDiscovery<G, L>
where
    L: DashboardDiscovery,
{
    pub(super) const fn from_parts(
        daemon: Option<DaemonClient>,
        guard: Option<G>,
        local: L,
    ) -> Self {
        Self {
            _daemon_guard: guard,
            fallback: FallbackDiscovery::new(DaemonStateDiscovery { daemon }, local),
            capture_content: false,
            dashboard_binding_installed: false,
            terminal_states: TerminalStateTracker::new(),
        }
    }
}

impl<G, L> Drop for ProductionDiscovery<G, L> {
    fn drop(&mut self) {
        if self.dashboard_binding_installed {
            SystemTmuxCommand.remove_dashboard_binding();
        }
    }
}

impl<G, L> DashboardDiscovery for ProductionDiscovery<G, L>
where
    L: DashboardDiscovery,
{
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        let mut rows = self.fallback.refresh()?;
        if self.capture_content {
            hydrate_pane_content(&mut rows, &BTreeSet::new(), &mut self.terminal_states);
        }
        Ok(rows)
    }

    fn refresh_visible(
        &mut self,
        visible_agent_ids: &BTreeSet<String>,
    ) -> io::Result<Vec<DashboardRow>> {
        let mut rows = self.fallback.refresh()?;
        if self.capture_content {
            hydrate_pane_content(&mut rows, visible_agent_ids, &mut self.terminal_states);
        }
        Ok(rows)
    }

    fn forward_key(&mut self, pane_id: &str, event: &Event) -> io::Result<()> {
        let Event::Key(key) = event else {
            return Ok(());
        };
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        let Some(tmux_key) = tmux_key_name(key.code, key.modifiers) else {
            return Ok(());
        };
        SystemTmuxCommand
            .send_key(pane_id, &tmux_key)
            .map_err(io::Error::other)
    }

    fn switch_client_to_pane(&mut self, pane_id: &str) -> io::Result<()> {
        SystemTmuxCommand
            .switch_client_to_pane(pane_id)
            .map_err(io::Error::other)
    }
}

fn tmux_key_name(code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char(character) if character.is_ascii() => Some(format!("C-{character}")),
            _ => None,
        };
    }
    match code {
        KeyCode::Char(character) => Some(character.to_string()),
        KeyCode::Enter => Some("Enter".to_owned()),
        KeyCode::Backspace => Some("BSpace".to_owned()),
        KeyCode::Tab => Some("Tab".to_owned()),
        KeyCode::Up => Some("Up".to_owned()),
        KeyCode::Down => Some("Down".to_owned()),
        KeyCode::Left => Some("Left".to_owned()),
        KeyCode::Right => Some("Right".to_owned()),
        KeyCode::Home => Some("Home".to_owned()),
        KeyCode::End => Some("End".to_owned()),
        KeyCode::PageUp => Some("PageUp".to_owned()),
        KeyCode::PageDown => Some("PageDown".to_owned()),
        _ => None,
    }
}

fn hydrate_pane_content(
    rows: &mut [DashboardRow],
    visible_agent_ids: &BTreeSet<String>,
    terminal_states: &mut TerminalStateTracker,
) {
    let tmux = SystemTmuxCommand;
    let current_agent_ids = rows
        .iter()
        .map(|row| row.agent_id().to_owned())
        .collect::<BTreeSet<_>>();
    terminal_states.retain(&current_agent_ids);
    for row in rows {
        let agent_id = row.agent_id().to_owned();
        if let Ok(content) = tmux.capture_pane(row.pane_id()) {
            let content = normalize_preview_ansi(&sanitize_pane_content(&content));
            if visible_agent_ids.is_empty() || visible_agent_ids.contains(row.agent_id()) {
                row.set_content(content.clone());
            }
            if let Some(state) = classify_terminal_state(row.client(), &content) {
                let preserves_wait = row.state().starts_with("waiting_") && state == "idle";
                if !preserves_wait && let Some(stable) = terminal_states.observe(&agent_id, state) {
                    row.set_state(stable);
                }
            }
        } else if let Some(stable) = terminal_states.stable(&agent_id) {
            row.set_state(stable);
        }
    }
}

/// Remove terminal hyperlink controls and neutralize underline attributes.
/// `OpenCode` uses OSC-8 links in its rendered output; `ansi_to_tui` can retain
/// that underline state after a truncated/cropped link sequence.
fn normalize_preview_ansi(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            normalized.push(character);
            continue;
        }

        match chars.peek() {
            Some(']') => {
                chars.next();
                while let Some(next) = chars.next() {
                    if next == '\u{7}' {
                        break;
                    }
                    if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            Some('[') => {
                let mut sequence = String::from("\u{1b}[");
                chars.next();
                for next in chars.by_ref() {
                    sequence.push(next);
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                let params = sequence
                    .strip_prefix("\u{1b}[")
                    .unwrap_or_default()
                    .trim_end_matches('m');
                if sequence.ends_with('m')
                    && params.split(';').any(|part| part == "4" || part == "21")
                {
                    normalized.push_str("\u{1b}[24m");
                } else {
                    normalized.push_str(&sequence);
                }
            }
            _ => normalized.push(character),
        }
    }
    normalized.push_str("\u{1b}[24m");
    normalized
}

/// Apply agent-terminal fallbacks when hook/marker evidence is absent.
/// A live agent process alone is not enough to call a session working: most
/// interactive agent processes remain alive at their idle composer.
fn classify_terminal_state(client: &str, content: &str) -> Option<&'static str> {
    let client = client.trim().to_ascii_lowercase();
    let client = client.strip_suffix(".exe").unwrap_or(&client);
    let content = strip_ansi_sequences(content).to_ascii_lowercase();
    let waiting_markers: &[&str] = match client {
        "opencode" => &["allow once", "allow always", "reject", "[y/n]", "(y/n)"],
        "codex" => &[
            "would you like to run the following command?",
            "would you like to make the following edits?",
            "would you like to grant these permissions?",
            "press enter to confirm or esc to cancel",
            "allow command?",
        ],
        "claude" => &["requires approval", "permission rule"],
        _ => &[],
    };
    let waiting_line = content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            waiting_markers
                .iter()
                .any(|marker| line.contains(marker))
                .then_some(index)
        })
        .last();

    // Only downgrade a live process when the pane exposes a known composer
    // footer. Unknown terminal layouts retain the process fallback.
    let idle_line = content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            (line.contains("ready for your next message")
                || (line.contains("ctrl+p") && line.contains("commands")))
            .then_some(index)
        })
        .last();
    let codex_composer = client == "codex"
        && content
            .lines()
            .any(|line| line.trim_start().starts_with('›'));
    let markers: &[&str] = match client {
        "opencode" => &[
            "esc interrupt",
            "esc again to interrupt",
            "ctrl+c to interrupt",
        ],
        "codex" => &["esc to interrupt", "ctrl+c to interrupt"],
        _ => &[],
    };
    let working_line = content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            markers
                .iter()
                .any(|marker| line.contains(marker))
                .then_some(index)
        })
        .last();
    if let Some(waiting_line) = waiting_line {
        let latest_other_line = idle_line.into_iter().chain(working_line).max();
        if latest_other_line.is_none_or(|line| waiting_line > line) {
            return Some("waiting_permission");
        }
    }
    if let Some(working_line) = working_line {
        if idle_line.is_some_and(|idle_line| working_line < idle_line) {
            return Some("idle");
        }
        return Some("working");
    }
    if idle_line.is_some() || codex_composer {
        return Some("idle");
    }
    match client {
        // These interactive processes remain alive at rest. Once a pane was
        // captured successfully, absence of a current waiting/interrupt
        // marker means the agent is idle rather than actively working.
        "opencode" | "codex" => Some("idle"),
        _ => None,
    }
}

fn strip_ansi_sequences(content: &str) -> String {
    let mut clean = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            clean.push(character);
            continue;
        }

        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        }
    }
    clean
}

fn sanitize_pane_content(content: &str) -> String {
    let sanitized: String = content
        .chars()
        .filter(|character| {
            matches!(*character, '\n' | '\r' | '\t' | '\u{1b}') || !character.is_control()
        })
        .collect();

    if sanitized.len() <= MAX_PANE_CONTENT_CHARS {
        return sanitized;
    }

    // Keep the newest screen content. The composer and status footer are at
    // the bottom of the pane, and truncating from the front is what caused
    // OpenCode's input area to disappear on larger captures.
    let mut start = sanitized.len().saturating_sub(MAX_PANE_CONTENT_CHARS);
    while !sanitized.is_char_boundary(start) {
        start = start.saturating_sub(1);
    }
    if let Some(offset) = sanitized[start..].find('\n') {
        start = start.saturating_add(offset.saturating_add(1));
    }
    format!("\u{1b}[0m{}", &sanitized[start..])
}

#[cfg(test)]
mod tests {
    use super::{
        TerminalStateTracker, classify_terminal_state, normalize_preview_ansi,
        sanitize_pane_content,
    };

    #[test]
    fn terminal_state_tracker_debounces_idle_and_retains_stable_state() {
        let mut tracker = TerminalStateTracker::new();

        assert_eq!(tracker.observe("pane:%1", "working"), Some("working"));
        assert_eq!(tracker.observe("pane:%1", "idle"), Some("working"));
        assert_eq!(tracker.observe("pane:%1", "working"), Some("working"));
        assert_eq!(tracker.observe("pane:%1", "idle"), Some("working"));
        assert_eq!(tracker.observe("pane:%1", "idle"), Some("idle"));
        assert_eq!(tracker.stable("pane:%1"), Some("idle"));
    }

    #[test]
    fn idle_agent_footer_is_reported_as_idle() {
        assert_eq!(
            classify_terminal_state("opencode", "ctrl+p commands"),
            Some("idle")
        );
        assert_eq!(
            classify_terminal_state("codex", "ctrl+p commands"),
            Some("idle")
        );
        assert_eq!(
            classify_terminal_state("codex", "›  Type a message"),
            Some("idle")
        );
    }

    #[test]
    fn agent_working_footer_is_reported_as_working() {
        assert_eq!(
            classify_terminal_state("opencode", "\u{1b}[32mEsc interrupt\u{1b}[0m"),
            Some("working")
        );
    }

    #[test]
    fn idle_footer_wins_over_stale_working_text() {
        assert_eq!(
            classify_terminal_state("opencode", "previous esc interrupt\nctrl+p commands"),
            Some("idle")
        );
        assert_eq!(
            classify_terminal_state("opencode", "esc interrupt    ctrl+p commands"),
            Some("working")
        );
    }

    #[test]
    fn current_working_line_wins_over_stale_permission_text() {
        assert_eq!(
            classify_terminal_state(
                "opencode",
                "allow once\nold output\nesc interrupt    ctrl+p commands"
            ),
            Some("working")
        );
    }

    #[test]
    fn codex_working_marker_wins_over_persistent_composer() {
        assert_eq!(
            classify_terminal_state(
                "codex",
                "Working (30s • esc to interrupt)\n› Ask Codex to do anything"
            ),
            Some("working")
        );
    }

    #[test]
    fn launcher_suffix_uses_the_same_idle_status_rules() {
        assert_eq!(
            classify_terminal_state("opencode.exe", "ctrl+p commands"),
            Some("idle")
        );
    }

    #[test]
    fn captured_agent_without_active_marker_is_reported_as_idle() {
        assert_eq!(
            classify_terminal_state("opencode", "completed agent output"),
            Some("idle")
        );
        assert_eq!(
            classify_terminal_state("codex", "completed agent output"),
            Some("idle")
        );
        assert_eq!(classify_terminal_state("gemini", "agent output"), None);
    }

    #[test]
    fn agent_permission_prompt_is_reported_as_waiting() {
        assert_eq!(
            classify_terminal_state("codex", "Press enter to confirm or esc to cancel"),
            Some("waiting_permission")
        );
    }

    #[test]
    fn oversized_capture_keeps_the_bottom_composer() {
        let content = format!("{}\ncomposer input", "x".repeat(40_000));
        let captured = sanitize_pane_content(&content);

        assert!(captured.contains("composer input"));
        assert!(captured.len() <= 32_000 + 4);
    }

    #[test]
    fn opencode_link_sequences_do_not_leave_the_preview_underlined() {
        let content = "\u{1b}]8;;file:///tmp/a\u{7}draft\u{1b}]8;;\u{7} plain";
        let normalized = normalize_preview_ansi(content);

        assert_eq!(normalized, "draft plain\u{1b}[24m");
    }
}

#[derive(Debug)]
pub(super) struct FallbackDiscovery<P, F> {
    pub(super) primary: P,
    pub(super) fallback: F,
}

impl<P, F> FallbackDiscovery<P, F> {
    pub(super) const fn new(primary: P, fallback: F) -> Self {
        Self { primary, fallback }
    }
}

impl<P, F> DashboardDiscovery for FallbackDiscovery<P, F>
where
    P: DashboardDiscovery,
    F: DashboardDiscovery,
{
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        self.primary
            .refresh()
            .or_else(|_error| self.fallback.refresh())
    }
}

#[derive(Debug)]
struct DaemonStateDiscovery {
    daemon: Option<DaemonClient>,
}

impl DashboardDiscovery for DaemonStateDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        let daemon = self
            .daemon
            .ok_or_else(|| io::Error::other("daemon address unavailable"))?;
        daemon.fetch_rows().map_err(io::Error::other)
    }
}

#[derive(Debug)]
pub(super) struct LocalDiscovery {
    service: DiscoveryService<SystemTmuxCommand>,
}

impl LocalDiscovery {
    fn new() -> Self {
        Self {
            service: DiscoveryService::new(SystemTmuxCommand),
        }
    }
}

impl DashboardDiscovery for LocalDiscovery {
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        let snapshot = self.service.refresh().map_err(io::Error::other)?;
        Ok(projection::project_snapshot(&snapshot)
            .iter()
            .map(DashboardRow::from_projection)
            .collect())
    }
}

pub(super) fn normalize_daemon_guard<G>(result: Result<Option<G>, AutostartError>) -> Option<G> {
    result.ok().flatten()
}

fn default_daemon_addr() -> Option<SocketAddr> {
    DEFAULT_DAEMON_ADDR.parse().ok()
}
