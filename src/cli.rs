//! Command-line interface for the agentmux dashboard.

use std::io::IsTerminal;
use std::io::{self, Write};

use std::net::SocketAddr;
use std::time::Duration;

use clap::{ArgAction, Args, Parser, Subcommand};

use crate::daemon::{api::DEFAULT_DAEMON_ADDR, runner::DaemonConfig};
use crate::inspect;
use crate::runner;
use crate::tmux::SystemTmuxCommand;

/// Parsed top-level CLI arguments.
#[derive(Debug, Parser)]
#[non_exhaustive]
#[command(
    name = "agentmux",
    version,
    about = "Terminal dashboard for coding-agent workflows.",
    long_about = None
)]
pub struct Cli {
    /// Optional command to run.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Supported CLI commands.
#[derive(Debug, Subcommand)]
#[non_exhaustive]
pub enum Command {
    /// Open the dashboard.
    Dashboard(DashboardArgs),
    /// Inspect current tmux-derived agent state once.
    Inspect,
    /// Run the loopback-only live daemon.
    Daemon(DaemonArgs),
}

/// Arguments for the dashboard command.
#[derive(Clone, Copy, Debug, Args)]
pub struct DashboardArgs {
    /// Skip dashboard daemon autostart and use only an already-running daemon.
    #[arg(long = "no-daemon", action = ArgAction::SetFalse, default_value_t = true)]
    auto_start_daemon: bool,
}

impl DashboardArgs {
    const fn default_dashboard() -> Self {
        Self {
            auto_start_daemon: true,
        }
    }
}

/// Arguments for the live daemon command.
#[derive(Debug, Args)]
pub struct DaemonArgs {
    /// Loopback bind address for the local daemon API.
    #[arg(long, default_value = DEFAULT_DAEMON_ADDR)]
    bind: SocketAddr,
}

impl Cli {
    /// Run the selected CLI command.
    pub fn run(self) -> color_eyre::Result<()> {
        match self.command {
            Some(Command::Inspect) => {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                finish_inspect_command(
                    inspect::run_inspect(SystemTmuxCommand, &mut stdout),
                    &mut stdout,
                )
            }
            Some(Command::Daemon(args)) => {
                let config = DaemonConfig::new(args.bind, Duration::from_secs(1));
                crate::daemon::runner::run(config)?;
                Ok(())
            }
            None if io::stdin().is_terminal() && io::stdout().is_terminal() => {
                let args = DashboardArgs::default_dashboard();
                runner::run_dashboard_with_options(
                    runner::DashboardOptions::from_auto_start_daemon(args.auto_start_daemon),
                )?;
                Ok(())
            }
            Some(Command::Dashboard(args))
                if io::stdin().is_terminal() && io::stdout().is_terminal() =>
            {
                runner::run_dashboard_with_options(
                    runner::DashboardOptions::from_auto_start_daemon(args.auto_start_daemon),
                )?;
                Ok(())
            }
            None | Some(Command::Dashboard(_)) => Ok(()),
        }
    }
}

fn finish_inspect_command(
    inspect_result: Result<(), inspect::InspectError>,
    stdout: &mut impl Write,
) -> color_eyre::Result<()> {
    match inspect_result {
        Ok(()) => {}
        Err(error) if error.is_broken_pipe() => return Ok(()),
        Err(error) => return Err(error.into()),
    }

    match stdout.flush() {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::cli::finish_inspect_command;
    use crate::inspect::InspectError;
    use crate::tmux::TmuxError;

    #[test]
    fn given_inspect_broken_pipe_when_finished_then_cli_succeeds() {
        // Given: inspect returned a BrokenPipe write error.
        let inspect_result = Err(inspect_io_error(io::ErrorKind::BrokenPipe));
        let mut stdout = FlushWriter::ok();

        // When: the CLI boundary finishes the inspect command.
        let result = finish_inspect_command(inspect_result, &mut stdout);

        // Then: BrokenPipe is treated as successful early termination.
        assert!(result.is_ok());
    }

    #[test]
    fn given_inspect_permission_denied_when_finished_then_cli_fails() {
        // Given: inspect returned a non-BrokenPipe write error.
        let inspect_result = Err(inspect_io_error(io::ErrorKind::PermissionDenied));
        let mut stdout = FlushWriter::ok();

        // When: the CLI boundary finishes the inspect command.
        let result = finish_inspect_command(inspect_result, &mut stdout);

        // Then: non-BrokenPipe inspect I/O errors remain failures.
        assert!(result.is_err());
    }

    #[test]
    fn given_inspect_tmux_error_when_finished_then_cli_fails() {
        // Given: inspect returned a tmux discovery error.
        let inspect_result = Err(InspectError::Tmux {
            source: TmuxError::CommandFailed { status: Some(1) },
        });
        let mut stdout = FlushWriter::ok();

        // When: the CLI boundary finishes the inspect command.
        let result = finish_inspect_command(inspect_result, &mut stdout);

        // Then: tmux failures remain CLI failures.
        assert!(result.is_err());
    }

    #[test]
    fn given_flush_broken_pipe_when_finished_then_cli_succeeds() {
        // Given: inspect succeeded but stdout flush hits BrokenPipe.
        let mut stdout = FlushWriter::err(io::ErrorKind::BrokenPipe);

        // When: the CLI boundary flushes stdout.
        let result = finish_inspect_command(Ok(()), &mut stdout);

        // Then: flush BrokenPipe is treated as successful early termination.
        assert!(result.is_ok());
    }

    #[test]
    fn given_flush_permission_denied_when_finished_then_cli_fails() {
        // Given: inspect succeeded but stdout flush hits a non-BrokenPipe error.
        let mut stdout = FlushWriter::err(io::ErrorKind::PermissionDenied);

        // When: the CLI boundary flushes stdout.
        let result = finish_inspect_command(Ok(()), &mut stdout);

        // Then: non-BrokenPipe flush errors remain failures.
        assert!(result.is_err());
    }

    fn inspect_io_error(kind: io::ErrorKind) -> InspectError {
        InspectError::Io {
            source: io::Error::new(kind, "inspect writer failed"),
        }
    }

    struct FlushWriter {
        flush_result: io::Result<()>,
    }

    impl FlushWriter {
        const fn ok() -> Self {
            Self {
                flush_result: Ok(()),
            }
        }

        fn err(kind: io::ErrorKind) -> Self {
            Self {
                flush_result: Err(io::Error::new(kind, "stdout flush failed")),
            }
        }
    }

    impl io::Write for FlushWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            match &self.flush_result {
                Ok(()) => Ok(()),
                Err(error) => Err(io::Error::new(error.kind(), "stdout flush failed")),
            }
        }
    }
}
