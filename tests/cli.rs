//! CLI contract tests for the `agentmux` binary.
//!
//! These tests describe the user-facing CLI surface expected from the
//! dashboard foundation.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn given_help_flag_when_invoked_then_stdout_mentions_agentmux_and_dashboard() {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux").expect("binary exists");

    // When: the help flag is requested.
    let assert = command.arg("--help").assert();

    assert.success().stdout(
        contains("Terminal dashboard for coding-agent workflows")
            .and(contains("Open the dashboard")),
    );
}

#[test]
fn given_help_flag_when_invoked_then_stdout_mentions_inspect() {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux").expect("binary exists");

    // When: the help flag is requested.
    let assert = command.arg("--help").assert();

    // Then: the one-shot inspect command is discoverable.
    assert
        .success()
        .stdout(contains("Inspect current tmux-derived agent state once"));
}

#[test]
fn given_dashboard_command_when_invoked_then_process_exits_successfully() {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux").expect("binary exists");

    // When: the dashboard command is requested.
    let assert = command.arg("dashboard").assert();

    // Then: the command exits successfully.
    assert.success();
}

#[test]
fn given_tmux_missing_when_inspect_runs_then_process_fails_without_stdout() {
    // Given: the child process cannot find tmux on PATH.
    let mut command = Command::cargo_bin("agentmux").expect("binary exists");

    // When: the inspect command runs.
    let assert = command.arg("inspect").env("PATH", "/nonexistent").assert();

    // Then: the binary exits nonzero, explains the tmux execution failure, and emits no stdout.
    assert
        .failure()
        .stdout("")
        .stderr(contains("failed to execute tmux"));
}

#[test]
fn given_unsupported_command_when_invoked_then_process_fails_with_usage() {
    // Given: the compiled CLI binary.
    let mut command = Command::cargo_bin("agentmux").expect("binary exists");

    // When: an unsupported command is requested.
    let assert = command.arg("unsupported").assert();

    // Then: the command fails and prints usage guidance.
    assert.failure().stderr(contains("Usage"));
}
