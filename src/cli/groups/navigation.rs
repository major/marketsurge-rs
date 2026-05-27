//! Navigation group: coach and site navigation trees.

use clap::Subcommand;

use crate::cli::TreeArgs;
use crate::cli::commands;

/// Navigation subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch the coach tree of watchlists and screens.
    #[command(after_help = "Examples:\n  marketsurge-agent navigation coach")]
    Coach,

    /// Fetch the site navigation tree.
    #[command(after_help = "Examples:\n  marketsurge-agent navigation nav")]
    Nav,
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    let tree_cmd = match cmd {
        Cmd::Coach => commands::tree::TreeCommand::Coach,
        Cmd::Nav => commands::tree::TreeCommand::Nav,
    };
    commands::tree::handle(&TreeArgs { command: tree_cmd }, fields).await
}
