//! Daemon command runner.

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::tmux::SystemTmuxCommand;

use super::DiscoveryService;
use super::api::{self, ApiError, SharedDaemonState};
use super::state::DaemonState;

/// Daemon runtime configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DaemonConfig {
    /// Loopback bind address.
    pub bind: SocketAddr,
    /// Fallback discovery refresh interval.
    pub refresh_interval: Duration,
}

impl DaemonConfig {
    /// Create daemon config for a loopback bind address.
    #[must_use]
    pub const fn new(bind: SocketAddr, refresh_interval: Duration) -> Self {
        Self {
            bind,
            refresh_interval,
        }
    }
}

/// Run the live local daemon until the process exits.
pub fn run(config: DaemonConfig) -> Result<(), ApiError> {
    api::validate_loopback_addr(config.bind)?;
    let listener = TcpListener::bind(config.bind)?;
    let state = Arc::new(SharedDaemonState::new(DaemonState::default()));
    spawn_discovery_loop(Arc::clone(&state), config.refresh_interval);
    api::serve(&listener, &state)
}

fn spawn_discovery_loop(state: Arc<SharedDaemonState>, refresh_interval: Duration) {
    thread::spawn(move || {
        let mut discovery = DiscoveryService::new(SystemTmuxCommand);
        loop {
            if let Ok(snapshot) = discovery.refresh() {
                let _result =
                    state.update(|daemon_state| daemon_state.reconcile_fallback(snapshot));
            }
            thread::sleep(refresh_interval);
        }
    });
}
