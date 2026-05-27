//! Industry group: RS ratings and overview data.

use clap::Subcommand;

use crate::cli::commands;
use crate::cli::{IndustryArgs, SymbolsArgs};

/// Industry subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch industry group relative strength ratings for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent industry rs AAPL MSFT")]
    Rs(SymbolsArgs),

    /// Fetch industry rankings, sector, and breadth data for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent industry overview AAPL MSFT")]
    Overview(SymbolsArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    let industry_cmd = match cmd {
        Cmd::Rs(args) => commands::industry::IndustryCommand::Rs(args.clone()),
        Cmd::Overview(args) => commands::industry::IndustryCommand::Overview(args.clone()),
    };
    commands::industry::handle(
        &IndustryArgs {
            command: industry_cmd,
        },
        fields,
    )
    .await
}
