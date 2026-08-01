//! Dashboard discovery implementations.

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

use super::{DashboardDiscovery, DashboardOptions};

const DAEMON_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(super) struct ProductionDiscovery<G = DashboardDaemonGuard<Child>, L = LocalDiscovery> {
    _daemon_guard: Option<G>,
    fallback: FallbackDiscovery<DaemonStateDiscovery, L>,
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
        Self::from_parts(daemon, guard, LocalDiscovery::new())
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
        }
    }
}

impl<G, L> DashboardDiscovery for ProductionDiscovery<G, L>
where
    L: DashboardDiscovery,
{
    fn refresh(&mut self) -> io::Result<Vec<DashboardRow>> {
        self.fallback.refresh()
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
