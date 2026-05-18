//! MarketSurge CLI agent.
//!
//! This project is unofficial and is not affiliated with, endorsed by, or
//! sponsored by [MarketSurge](https://marketsurge.investors.com).

#![deny(missing_docs)]

/// Clap command tree and top-level argument structs.
pub mod cli;
/// Command handlers for each CLI subcommand group.
pub mod commands;
/// Shared utilities: auth helpers and error mapping.
pub mod common;
/// JSON output formatting and field selection.
pub mod output;

use clap::Parser;

use crate::cli::{Cli, Commands};

/// Parses CLI arguments, routes to the appropriate command handler, and returns
/// the process exit code.
pub async fn run() -> i32 {
    let cli = Cli::parse();

    let json_table = !cli.json_objects;

    match &cli.command {
        Commands::AdhocScreen(args) => commands::adhoc_screen::handle(args, json_table).await,
        Commands::Chart(args) => commands::chart::handle(args, json_table).await,
        Commands::Fundamentals(args) => commands::fundamentals::handle(args, json_table).await,
        Commands::Industry(args) => commands::industry::handle(args, json_table).await,
        Commands::MarketData(args) => commands::market_data::handle(args, json_table).await,
        Commands::Markups(args) => commands::markups::handle(args, json_table).await,
        Commands::Ownership(args) => commands::ownership::handle(args, json_table).await,
        Commands::Ratings(args) => commands::ratings::handle(args, json_table).await,
        Commands::Screen(args) => commands::screen::handle(args, json_table).await,
        Commands::Tree(args) => commands::tree::handle(args, json_table).await,
        Commands::Watchlist(args) => commands::watchlist::handle(args, json_table).await,
        Commands::Completions(args) => {
            commands::completions::handle(args);
            0
        }
    }
}
