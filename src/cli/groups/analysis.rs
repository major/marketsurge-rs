//! Analysis group: fundamentals and RS ratings.

use clap::Subcommand;

use crate::cli::SymbolsArgs;
use crate::cli::commands;

/// Analysis subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch EPS, sales, and estimate fundamentals for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent analysis fundamentals AAPL MSFT")]
    Fundamentals(SymbolsArgs),

    /// Fetch relative strength ratings for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent analysis ratings AAPL MSFT")]
    Ratings(SymbolsArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    match cmd {
        Cmd::Fundamentals(args) => commands::fundamentals::handle(args, fields).await,
        Cmd::Ratings(args) => commands::ratings::handle(args, fields).await,
    }
}
