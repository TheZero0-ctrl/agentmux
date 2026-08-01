//! Dashboard CLI contract tests.

use std::error::Error;

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn given_dashboard_help_when_invoked_then_stdout_mentions_no_daemon() -> Result<(), Box<dyn Error>>
{
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: dashboard help is requested.
    let assert = command.args(["dashboard", "--help"]).assert();

    // Then: the dashboard exposes the opt-out flag for daemon autostart.
    assert.success().stdout(contains("--no-daemon"));
    Ok(())
}

#[test]
fn given_no_tty_and_no_daemon_when_dashboard_runs_then_it_succeeds_without_output()
-> Result<(), Box<dyn Error>> {
    // Given: the dashboard command is run without an interactive terminal.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: daemon autostart is disabled explicitly.
    let assert = command
        .args(["dashboard", "--no-daemon"])
        .write_stdin("")
        .assert();

    // Then: non-interactive dashboard remains a successful no-op.
    assert.success().stdout("").stderr("");
    Ok(())
}
