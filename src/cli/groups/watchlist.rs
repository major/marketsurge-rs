//! Watchlist group: list watchlists, read symbols, or screen symbols.

use clap::Subcommand;

use crate::cli::WatchlistArgs;
use crate::cli::commands;
use crate::cli::commands::watchlist::{
    WatchlistListArgs, WatchlistScreenArgs, WatchlistSymbolsArgs,
};

/// Watchlist subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List saved watchlists.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist list\n  marketsurge-agent watchlist list --query ibd"
    )]
    List(WatchlistListArgs),

    /// Fetch symbols in a watchlist by ID.
    #[command(after_help = "Examples:\n  marketsurge-agent watchlist symbols 12345")]
    Symbols(WatchlistSymbolsArgs),

    /// Screen symbols with selected MarketSurge data columns.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist screen AAPL MSFT\n  marketsurge-agent watchlist screen AAPL --columns Symbol,EPSRating,RSRating"
    )]
    Screen(WatchlistScreenArgs),
}

/// Dispatch to the appropriate command handler.
#[cfg(not(coverage))]
pub(crate) async fn dispatch(cmd: &Cmd, fields: &[String]) -> i32 {
    let watchlist_cmd = match cmd {
        Cmd::List(args) => commands::watchlist::WatchlistCommand::List(args.clone()),
        Cmd::Symbols(args) => commands::watchlist::WatchlistCommand::Symbols(args.clone()),
        Cmd::Screen(args) => commands::watchlist::WatchlistCommand::Screen(args.clone()),
    };
    commands::watchlist::handle(
        &WatchlistArgs {
            command: watchlist_cmd,
        },
        fields,
    )
    .await
}
