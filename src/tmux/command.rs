use std::process::Command;

use super::{TmuxError, TmuxOutput};

/// Injectable boundary for collecting tmux pane rows.
pub trait TmuxCommand {
    /// Run `tmux list-panes` with the supplied format string.
    ///
    /// # Errors
    /// Returns [`TmuxError`] when command execution fails.
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError>;
}

/// Production tmux command runner.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct SystemTmuxCommand;

impl TmuxCommand for SystemTmuxCommand {
    fn list_panes(&self, format: &str) -> Result<TmuxOutput, TmuxError> {
        let output = Command::new("tmux")
            .args(["list-panes", "-a", "-F", format])
            .output()
            .map_err(|source| TmuxError::CommandIo { source })?;

        if output.status.success() {
            return Ok(TmuxOutput::new(
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ));
        }

        Err(TmuxError::command_failed(
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).as_ref(),
        ))
    }
}
