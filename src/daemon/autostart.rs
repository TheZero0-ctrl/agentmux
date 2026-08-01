//! Dashboard-owned daemon autostart lifecycle.

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::api::validate_loopback_addr;

const STARTUP_PROBE_ATTEMPTS: usize = 20;
const STARTUP_PROBE_INTERVAL: Duration = Duration::from_millis(25);

/// Dashboard daemon health classification from `GET /health`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DaemonHealth {
    /// An agentmux daemon answered `/health` successfully.
    Healthy,
    /// Nothing accepted a connection on the daemon address.
    Unreachable,
    /// Something answered, but it was not a valid agentmux daemon.
    OccupiedInvalid,
}

/// Sanitized dashboard daemon autostart error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AutostartError {
    /// Dashboard daemon bind was not loopback-only.
    NonLoopbackBind,
    /// The daemon child could not be spawned.
    SpawnFailed,
}

impl fmt::Display for AutostartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonLoopbackBind => write!(formatter, "dashboard daemon bind must be loopback"),
            Self::SpawnFailed => write!(formatter, "dashboard daemon autostart failed"),
        }
    }
}

impl std::error::Error for AutostartError {}

/// Child process operations needed by a dashboard-owned daemon guard.
pub trait DashboardDaemonChild {
    /// Ask the owned child to stop.
    fn kill(&mut self) -> io::Result<()>;

    /// Reap the owned child process.
    fn wait(&mut self) -> io::Result<()>;
}

impl DashboardDaemonChild for Child {
    fn kill(&mut self) -> io::Result<()> {
        self.kill()
    }

    fn wait(&mut self) -> io::Result<()> {
        self.wait().map(|_status| ())
    }
}

/// RAII guard that owns only the daemon child spawned for the dashboard.
#[derive(Debug)]
pub struct DashboardDaemonGuard<C: DashboardDaemonChild> {
    child: Option<C>,
}

impl<C: DashboardDaemonChild> DashboardDaemonGuard<C> {
    /// Create a guard for a dashboard-owned daemon child.
    pub const fn new(child: C) -> Self {
        Self { child: Some(child) }
    }
}

impl<C: DashboardDaemonChild> Drop for DashboardDaemonGuard<C> {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _kill_result = child.kill();
            let _wait_result = child.wait();
        }
    }
}

/// Probe daemon health at a loopback socket address.
pub trait HealthProbe {
    /// Classify the daemon endpoint using `/health`.
    fn classify(&self, addr: SocketAddr) -> DaemonHealth;
}

/// Standard-library TCP `/health` probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpHealthProbe {
    timeout: Duration,
}

impl TcpHealthProbe {
    /// Create a TCP health probe with bounded I/O.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl HealthProbe for TcpHealthProbe {
    fn classify(&self, addr: SocketAddr) -> DaemonHealth {
        classify_health(addr, self.timeout)
    }
}

/// Spawn a daemon from the current executable.
pub trait DashboardDaemonSpawner {
    /// Child handle returned by the spawner.
    type Child: DashboardDaemonChild;

    /// Start `agentmux daemon --bind <addr>`.
    fn spawn_daemon(&self, addr: SocketAddr) -> Result<Self::Child, AutostartError>;
}

/// Production dashboard daemon spawner.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct CurrentExecutableSpawner;

impl DashboardDaemonSpawner for CurrentExecutableSpawner {
    type Child = Child;

    fn spawn_daemon(&self, addr: SocketAddr) -> Result<Self::Child, AutostartError> {
        let executable = std::env::current_exe().map_err(|_error| AutostartError::SpawnFailed)?;
        Command::new(executable)
            .arg("daemon")
            .arg("--bind")
            .arg(addr.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_error| AutostartError::SpawnFailed)
    }
}

/// Ensure the dashboard has a daemon only when the endpoint is unreachable.
pub fn ensure_dashboard_daemon<P, S>(
    addr: SocketAddr,
    probe: &P,
    spawner: &S,
) -> Result<Option<DashboardDaemonGuard<S::Child>>, AutostartError>
where
    P: HealthProbe,
    S: DashboardDaemonSpawner,
{
    validate_loopback_addr(addr).map_err(|_error| AutostartError::NonLoopbackBind)?;
    match probe.classify(addr) {
        DaemonHealth::Healthy | DaemonHealth::OccupiedInvalid => Ok(None),
        DaemonHealth::Unreachable => {
            let guard = DashboardDaemonGuard::new(spawner.spawn_daemon(addr)?);
            wait_until_healthy(addr, probe)
                .then_some(guard)
                .map_or(Ok(None), |guard| Ok(Some(guard)))
        }
    }
}

fn wait_until_healthy(addr: SocketAddr, probe: &impl HealthProbe) -> bool {
    for _attempt in 0..STARTUP_PROBE_ATTEMPTS {
        if probe.classify(addr) == DaemonHealth::Healthy {
            return true;
        }
        std::thread::sleep(STARTUP_PROBE_INTERVAL);
    }
    false
}

fn classify_health(addr: SocketAddr, timeout: Duration) -> DaemonHealth {
    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(stream) => stream,
        Err(_error) => return DaemonHealth::Unreachable,
    };
    if stream.set_read_timeout(Some(timeout)).is_err()
        || stream.set_write_timeout(Some(timeout)).is_err()
        || stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: agentmux\r\nConnection: close\r\n\r\n")
            .is_err()
    {
        return DaemonHealth::OccupiedInvalid;
    }
    read_health_response(stream).map_or(DaemonHealth::OccupiedInvalid, |()| DaemonHealth::Healthy)
}

fn read_health_response(stream: TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if !status.starts_with("HTTP/1.1 200 ") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid health status",
        ));
    }
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
    }
    let mut body = String::new();
    reader.read_to_string(&mut body)?;
    if body.starts_with("agentmux daemon ok revision: ") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid health body",
        ))
    }
}

#[cfg(test)]
mod tests;
