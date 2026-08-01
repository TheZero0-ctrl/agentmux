use std::cell::{Cell, RefCell};
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use super::{
    AutostartError, DaemonHealth, DashboardDaemonChild, DashboardDaemonGuard,
    DashboardDaemonSpawner, HealthProbe, ensure_dashboard_daemon,
};

#[test]
fn given_healthy_daemon_when_ensured_then_no_child_is_spawned() -> io::Result<()> {
    // Given: the dashboard health probe sees an existing healthy daemon.
    let probe = StaticProbe(DaemonHealth::Healthy);
    let spawner = FakeSpawner::ok(ChildEvents::default());

    // When: dashboard daemon setup runs.
    let guard =
        ensure_dashboard_daemon(loopback_addr()?, &probe, &spawner).map_err(io::Error::other)?;

    // Then: the dashboard uses the existing daemon without owning a child.
    assert!(guard.is_none());
    assert_eq!(spawner.spawn_count(), 0);
    Ok(())
}

#[test]
fn given_unreachable_daemon_when_ensured_then_owned_child_is_spawned() -> io::Result<()> {
    // Given: the dashboard health probe cannot reach a daemon until after spawning.
    let probe = SequenceProbe::new([DaemonHealth::Unreachable, DaemonHealth::Healthy]);
    let spawner = FakeSpawner::ok(ChildEvents::default());

    // When: dashboard daemon setup runs.
    let guard =
        ensure_dashboard_daemon(loopback_addr()?, &probe, &spawner).map_err(io::Error::other)?;

    // Then: a dashboard-owned daemon guard is returned.
    assert!(guard.is_some());
    assert_eq!(spawner.spawn_count(), 1);
    Ok(())
}

#[test]
fn given_spawned_daemon_never_becomes_healthy_when_ensured_then_child_is_stopped() -> io::Result<()>
{
    // Given: spawning succeeds but the health probe never accepts the endpoint.
    let events = ChildEvents::default();
    let probe = StaticProbe(DaemonHealth::Unreachable);
    let spawner = FakeSpawner::ok(events.clone());

    // When: dashboard daemon setup runs.
    let guard =
        ensure_dashboard_daemon(loopback_addr()?, &probe, &spawner).map_err(io::Error::other)?;

    // Then: no guard is kept and the temporary child was stopped for local fallback.
    assert!(guard.is_none());
    assert_eq!(spawner.spawn_count(), 1);
    assert_eq!(events.snapshot(), ["kill", "wait"]);
    Ok(())
}

#[test]
fn given_occupied_invalid_port_when_ensured_then_no_child_is_spawned() -> io::Result<()> {
    // Given: the daemon port answers but not as an agentmux daemon.
    let probe = StaticProbe(DaemonHealth::OccupiedInvalid);
    let spawner = FakeSpawner::ok(ChildEvents::default());

    // When: dashboard daemon setup runs.
    let guard =
        ensure_dashboard_daemon(loopback_addr()?, &probe, &spawner).map_err(io::Error::other)?;

    // Then: the dashboard does not spawn over an occupied or invalid listener.
    assert!(guard.is_none());
    assert_eq!(spawner.spawn_count(), 0);
    Ok(())
}

#[test]
fn given_owned_guard_when_dropped_then_child_is_killed_and_waited() {
    // Given: a dashboard-owned child guard.
    let events = ChildEvents::default();
    let guard = DashboardDaemonGuard::new(FakeChild::new(events.clone()));

    // When: the dashboard guard is dropped.
    drop(guard);

    // Then: the child receives kill and wait, avoiding zombies.
    assert_eq!(events.snapshot(), ["kill", "wait"]);
}

#[test]
fn given_non_loopback_addr_when_ensured_then_spawn_is_rejected_without_details() -> io::Result<()> {
    // Given: a non-loopback daemon bind address.
    let probe = StaticProbe(DaemonHealth::Unreachable);
    let spawner = FakeSpawner::ok(ChildEvents::default());
    let addr = "0.0.0.0:47631"
        .parse::<SocketAddr>()
        .map_err(io::Error::other)?;

    // When: dashboard daemon setup runs.
    let error = ensure_dashboard_daemon(addr, &probe, &spawner)
        .map(|_guard| ())
        .err()
        .ok_or_else(|| io::Error::other("non-loopback bind was accepted"))?;

    // Then: the public error is sanitized and spawning was skipped.
    assert_eq!(error, AutostartError::NonLoopbackBind);
    assert_eq!(spawner.spawn_count(), 0);
    Ok(())
}

#[test]
fn given_spawn_failure_when_displayed_then_error_is_sanitized() {
    // Given: daemon spawning failed with private OS details underneath.
    let error = AutostartError::SpawnFailed;

    // When: the error is formatted for users.
    let display = error.to_string();

    // Then: no executable path, argv, or private value is exposed.
    assert_eq!(display, "dashboard daemon autostart failed");
    assert!(!display.contains("/home/"));
    assert!(!display.contains("daemon --bind"));
    assert!(!display.contains("token="));
}

#[derive(Clone, Copy)]
struct StaticProbe(DaemonHealth);

impl HealthProbe for StaticProbe {
    fn classify(&self, _addr: SocketAddr) -> DaemonHealth {
        self.0
    }
}

struct SequenceProbe<const N: usize> {
    index: Cell<usize>,
    states: [DaemonHealth; N],
}

impl<const N: usize> SequenceProbe<N> {
    const fn new(states: [DaemonHealth; N]) -> Self {
        Self {
            index: Cell::new(0),
            states,
        }
    }
}

impl<const N: usize> HealthProbe for SequenceProbe<N> {
    fn classify(&self, _addr: SocketAddr) -> DaemonHealth {
        let index = self.index.get().min(N.saturating_sub(1));
        self.index.set(index.saturating_add(1));
        self.states
            .get(index)
            .copied()
            .unwrap_or(DaemonHealth::Unreachable)
    }
}

#[derive(Clone, Default)]
struct ChildEvents(Rc<RefCell<Vec<&'static str>>>);

impl ChildEvents {
    fn push(&self, event: &'static str) {
        self.0.borrow_mut().push(event);
    }

    fn snapshot(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

struct FakeChild {
    events: ChildEvents,
}

impl FakeChild {
    fn new(events: ChildEvents) -> Self {
        Self { events }
    }
}

impl DashboardDaemonChild for FakeChild {
    fn kill(&mut self) -> io::Result<()> {
        self.events.push("kill");
        Ok(())
    }

    fn wait(&mut self) -> io::Result<()> {
        self.events.push("wait");
        Ok(())
    }
}

struct FakeSpawner {
    spawn_count: Cell<usize>,
    child_events: ChildEvents,
}

impl FakeSpawner {
    const fn ok(child_events: ChildEvents) -> Self {
        Self {
            spawn_count: Cell::new(0),
            child_events,
        }
    }

    fn spawn_count(&self) -> usize {
        self.spawn_count.get()
    }
}

impl DashboardDaemonSpawner for FakeSpawner {
    type Child = FakeChild;

    fn spawn_daemon(&self, _addr: SocketAddr) -> Result<Self::Child, AutostartError> {
        self.spawn_count
            .set(self.spawn_count.get().saturating_add(1));
        Ok(FakeChild::new(self.child_events.clone()))
    }
}

fn loopback_addr() -> io::Result<SocketAddr> {
    "127.0.0.1:47631".parse().map_err(io::Error::other)
}
