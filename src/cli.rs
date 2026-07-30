//! Command-line interface for the agentmux dashboard.

use std::io;
use std::io::IsTerminal;

use clap::{Parser, Subcommand};

use crate::runner;

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
#[allow(
    clippy::exhaustive_enums,
    reason = "the command set is intentionally crate-owned and tiny"
)]
pub enum Command {
    /// Open the dashboard.
    Dashboard,
}

impl Cli {
    /// Run the selected CLI command.
    pub fn run(self) -> io::Result<()> {
        match self.command {
            None | Some(Command::Dashboard)
                if io::stdin().is_terminal() && io::stdout().is_terminal() =>
            {
                runner::run_dashboard()
            }
            None | Some(Command::Dashboard) => Ok(()),
        }
    }
}
