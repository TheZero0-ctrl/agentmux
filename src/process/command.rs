/// Normalize a process command's executable token to its basename.
#[must_use]
pub fn normalize_command_name(command: &str) -> Option<&str> {
    let trimmed = command.split_whitespace().next()?;
    let basename = trimmed.rsplit('/').next()?;
    let without_login_prefix = basename
        .strip_prefix('-')
        .map_or(basename, |command_name| command_name);

    match without_login_prefix {
        "" => None,
        name => Some(name),
    }
}

pub(super) fn is_shell_command(command: &str) -> bool {
    matches!(
        command,
        "sh" | "bash"
            | "zsh"
            | "fish"
            | "nu"
            | "dash"
            | "ksh"
            | "mksh"
            | "csh"
            | "tcsh"
            | "elvish"
            | "xonsh"
            | "pwsh"
            | "powershell"
    )
}
