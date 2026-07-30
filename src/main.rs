#![forbid(unsafe_code)]

//! Binary entrypoint for agentmux.

use clap::Parser;

use agentmux::cli::Cli;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    cli.run()?;
    Ok(())
}
