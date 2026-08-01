use std::io;

use crate::app::DashboardRow;
use crate::daemon::autostart::AutostartError;

use super::DashboardDiscovery;
use super::discovery::{ProductionDiscovery, normalize_daemon_guard};
use super::test_support::FakeDiscovery;

#[test]
fn given_daemon_autostart_fails_when_dashboard_refreshes_then_local_fallback_still_works()
-> io::Result<()> {
    // Given: daemon autostart failed and local discovery can produce rows.
    let guard = normalize_daemon_guard::<()>(Err(AutostartError::SpawnFailed));
    let local_rows = vec![DashboardRow::for_test("pane:%1")];
    let local = FakeDiscovery::new([Ok(local_rows.clone())]);
    let mut discovery = ProductionDiscovery::from_parts(None, guard, local);

    // When: the dashboard refreshes.
    let rows = discovery.refresh()?;

    // Then: local fallback keeps the dashboard usable.
    assert_eq!(rows, local_rows);
    Ok(())
}
