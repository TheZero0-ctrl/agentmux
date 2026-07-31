//! CLI contract tests for the `agentmux` binary.
//!
//! These tests describe the user-facing CLI surface expected from the
//! dashboard foundation.

use std::error::Error;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

const PRIVATE_PATH: &str = "/home/alice/private-repo";
#[cfg(unix)]
const SECRET_TOKEN: &str = "token=super-secret";
#[cfg(unix)]
static NEXT_FAKE_TMUX_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn given_help_flag_when_invoked_then_stdout_mentions_agentmux_and_dashboard()
-> Result<(), Box<dyn Error>> {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: the help flag is requested.
    let assert = command.arg("--help").assert();

    assert.success().stdout(
        contains("Terminal dashboard for coding-agent workflows")
            .and(contains("Open the dashboard")),
    );
    Ok(())
}

#[test]
fn given_help_flag_when_invoked_then_stdout_mentions_inspect() -> Result<(), Box<dyn Error>> {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: the help flag is requested.
    let assert = command.arg("--help").assert();

    // Then: the one-shot inspect command is discoverable.
    assert
        .success()
        .stdout(contains("Inspect current tmux-derived agent state once"));
    Ok(())
}

#[test]
fn given_dashboard_command_when_invoked_then_process_exits_successfully()
-> Result<(), Box<dyn Error>> {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: the dashboard command is requested.
    let assert = command.arg("dashboard").assert();

    // Then: the command exits successfully.
    assert.success();
    Ok(())
}

#[test]
fn given_dashboard_command_without_tty_when_invoked_then_process_exits_without_output()
-> Result<(), Box<dyn Error>> {
    // Given: the compiled CLI binary with stdin closed to simulate non-interactive automation.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: the dashboard command is requested without a terminal.
    let assert = command.arg("dashboard").write_stdin("").assert();

    // Then: the dashboard degrades to a successful no-op with no terminal bytes emitted.
    assert.success().stdout("").stderr("");
    Ok(())
}

#[test]
fn given_tmux_missing_when_inspect_runs_then_process_fails_without_stdout()
-> Result<(), Box<dyn Error>> {
    // Given: the child process cannot find tmux on PATH.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: the inspect command runs.
    let assert = command.arg("inspect").env("PATH", "/nonexistent").assert();

    // Then: the binary exits nonzero, explains the tmux execution failure, and emits no stdout.
    assert
        .failure()
        .stdout("")
        .stderr(contains("failed to execute tmux").and(contains(PRIVATE_PATH).not()));
    Ok(())
}

#[test]
#[cfg(unix)]
fn given_secret_bearing_malformed_tmux_when_inspect_runs_then_process_fails_without_leaks()
-> Result<(), Box<dyn Error>> {
    // Given: a fake tmux executable emits a malformed secret-bearing row and secret stderr.
    let fake_bin = fake_tmux_bin("malformed")?;
    let mut command = Command::cargo_bin("agentmux")?;

    // When: inspect runs through the public subprocess boundary.
    let assert = command.arg("inspect").env("PATH", fake_bin.path()).assert();

    // Then: no partial stdout, raw tmux stdout, tmux stderr, path, or token reaches users.
    assert
        .failure()
        .stdout("")
        .stderr(contains("inspect discovery failed"))
        .stderr(contains(PRIVATE_PATH).not())
        .stderr(contains(SECRET_TOKEN).not())
        .stderr(contains("not-a-pid").not())
        .stderr(contains("tmux stderr").not());
    Ok(())
}

#[test]
#[cfg(unix)]
fn given_secret_bearing_tmux_stderr_when_inspect_runs_then_process_fails_without_leaks()
-> Result<(), Box<dyn Error>> {
    // Given: a fake tmux executable exits unsuccessfully after writing secret stderr.
    let fake_bin = fake_tmux_bin("failed")?;
    let mut command = Command::cargo_bin("agentmux")?;

    // When: inspect runs through the public subprocess boundary.
    let assert = command.arg("inspect").env("PATH", fake_bin.path()).assert();

    // Then: no stdout or raw tmux stderr reaches users.
    assert
        .failure()
        .stdout("")
        .stderr(contains("tmux list-panes failed"))
        .stderr(contains(PRIVATE_PATH).not())
        .stderr(contains(SECRET_TOKEN).not())
        .stderr(contains("tmux stderr").not());
    Ok(())
}

#[test]
fn given_unsupported_command_when_invoked_then_process_fails_with_usage()
-> Result<(), Box<dyn Error>> {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux")?;

    // When: an unsupported command is requested.
    let assert = command.arg("unsupported").assert();

    // Then: the command fails and prints usage guidance.
    assert.failure().stderr(contains("Usage"));
    Ok(())
}

#[cfg(unix)]
struct FakeTmuxBin {
    path: PathBuf,
}

#[cfg(unix)]
impl FakeTmuxBin {
    fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(unix)]
impl Drop for FakeTmuxBin {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn fake_tmux_bin(mode: &str) -> io::Result<FakeTmuxBin> {
    let path = create_unique_temp_dir(mode)?;
    let tmux_path = path.join("tmux");
    fs::write(&tmux_path, fake_tmux_script(mode))?;
    make_executable(&tmux_path)?;
    Ok(FakeTmuxBin { path })
}

#[cfg(unix)]
fn create_unique_temp_dir(mode: &str) -> io::Result<PathBuf> {
    loop {
        let nonce = NEXT_FAKE_TMUX_ID.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let path = std::env::temp_dir().join(format!("agentmux-cli-{mode}-{process_id}-{nonce}"));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(unix)]
fn fake_tmux_script(mode: &str) -> &'static str {
    match mode {
        "failed" => {
            r#"#!/bin/sh
	[ "$#" -eq 4 ] || exit 65
	case "$1 $2 $3 $4" in list-panes' '-a' '-F' '*pane_current_command*) ;; *) exit 65 ;; esac
		printf 'tmux stderr /home/alice/private-repo token=super-secret\n' >&2
		exit 7
		"#
        }
        "malformed" => {
            r#"#!/bin/sh
	[ "$#" -eq 4 ] || exit 65
	case "$1 $2 $3 $4" in list-panes' '-a' '-F' '*pane_current_command*) ;; *) exit 65 ;; esac
		printf 'work\0370\037api\037%%1\037not-a-pid\0370\037/home/alice/private-repo\037opencode token=super-secret\n'
		printf 'tmux stderr /home/alice/private-repo token=super-secret\n' >&2
		"#
        }
        _ => {
            r"#!/bin/sh
exit 64
"
        }
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
}
