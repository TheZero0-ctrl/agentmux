//! Daemon health classification integration tests.

use std::error::Error;
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use agentmux::daemon::api::{self, SharedDaemonState};
use agentmux::daemon::autostart::{DaemonHealth, HealthProbe, TcpHealthProbe};
use agentmux::daemon::state::DaemonState;

#[test]
fn given_agentmux_health_endpoint_when_classified_then_it_is_healthy() -> Result<(), Box<dyn Error>>
{
    // Given: a loopback agentmux API serving `/health`.
    let addr = spawn_agentmux_api()?;
    let probe = TcpHealthProbe::new(Duration::from_millis(500));

    // When: dashboard autostart classifies the endpoint.
    let health = probe.classify(addr);

    // Then: the endpoint is accepted as an existing healthy daemon.
    assert_eq!(health, DaemonHealth::Healthy);
    Ok(())
}

#[test]
fn given_no_listener_when_classified_then_it_is_unreachable() -> Result<(), Box<dyn Error>> {
    // Given: a loopback port with no listener.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);
    let probe = TcpHealthProbe::new(Duration::from_millis(100));

    // When: dashboard autostart classifies the endpoint.
    let health = probe.classify(addr);

    // Then: the endpoint is treated as missing, so autostart may spawn.
    assert_eq!(health, DaemonHealth::Unreachable);
    Ok(())
}

#[test]
fn given_non_agentmux_listener_when_classified_then_it_is_occupied_invalid()
-> Result<(), Box<dyn Error>> {
    // Given: something else is listening on the daemon port.
    let addr = spawn_invalid_listener()?;
    let probe = TcpHealthProbe::new(Duration::from_millis(500));

    // When: dashboard autostart classifies the endpoint.
    let health = probe.classify(addr);

    // Then: the endpoint is occupied and must not be replaced by a spawned daemon.
    assert_eq!(health, DaemonHealth::OccupiedInvalid);
    Ok(())
}

fn spawn_agentmux_api() -> Result<SocketAddr, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let shared = Arc::new(SharedDaemonState::new(DaemonState::default()));
    thread::spawn(move || {
        let _result = api::serve(&listener, &shared);
    });
    Ok(addr)
}

fn spawn_invalid_listener() -> Result<SocketAddr, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    thread::spawn(move || {
        if let Ok((mut stream, _peer)) = listener.accept() {
            let _result = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\nConnection: close\r\n\r\nnot agentmux",
            );
        }
    });
    Ok(addr)
}
