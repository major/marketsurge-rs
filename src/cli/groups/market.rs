//! Market data group: chart OHLCV bars and broad market snapshots.

use clap::Subcommand;

use crate::cli::commands;
use crate::cli::{ChartArgs, SymbolsArgs};

/// Market data subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch daily or weekly OHLCV bars for symbols.
    #[command(
        after_help = "Examples:\n  marketsurge-agent market chart AAPL MSFT\n  marketsurge-agent market chart --weekly AAPL"
    )]
    Chart(ChartArgs),

    /// Fetch broad rating, price, industry, and fundamental snapshot data.
    #[command(after_help = "Examples:\n  marketsurge-agent market snapshot AAPL MSFT")]
    Snapshot(SymbolsArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    match cmd {
        Cmd::Chart(args) => commands::chart::handle(args, fields).await,
        Cmd::Snapshot(args) => commands::market_data::handle(args, fields).await,
    }
}
