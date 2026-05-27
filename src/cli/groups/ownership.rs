//! Ownership group: fund ownership summaries and fund holdings.

use clap::Subcommand;

use crate::cli::commands;
use crate::cli::{OwnershipArgs, SymbolsArgs};

/// Ownership subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch quarterly fund ownership summary rows for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent ownership summary AAPL MSFT")]
    Summary(SymbolsArgs),

    /// Fetch individual fund holders and share history for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent ownership funds AAPL MSFT")]
    Funds(SymbolsArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    let ownership_cmd = match cmd {
        Cmd::Summary(args) => commands::ownership::OwnershipCommand::Summary(args.clone()),
        Cmd::Funds(args) => commands::ownership::OwnershipCommand::Funds(args.clone()),
    };
    commands::ownership::handle(
        &OwnershipArgs {
            command: ownership_cmd,
        },
        fields,
    )
    .await
}
