//! Auth command group: verify browser cookie and JWT readiness.

use clap::Subcommand;

/// Auth subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Check whether browser cookies and JWT are ready for API calls.
    #[command(after_help = "Examples:\n  marketsurge-agent auth status")]
    Status,
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    match cmd {
        Cmd::Status => crate::cli::commands::auth::handle(fields).await,
    }
}

#[cfg(test)]
mod tests {
    use clap::Subcommand;

    use super::Cmd;

    #[test]
    fn auth_command_tree_is_valid() {
        // Verify the command tree doesn't conflict with clap invariants.
        let cmd = Cmd::augment_subcommands(clap::Command::new("auth"));
        cmd.debug_assert();
    }
}
