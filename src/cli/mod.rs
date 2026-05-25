//! Command line parsing, dispatch, and output rendering.

mod args;
/// Command handlers for each CLI subcommand group.
pub mod commands;
/// Shared utilities: auth helpers and error mapping.
pub mod common;
/// JSON output formatting and field selection.
pub mod output;

pub use args::*;

use clap::Parser;

/// No-op stub so the binary compiles under `cargo-llvm-cov`.
///
/// Coverage instrumentation excludes the real `handle` functions (which call
/// `clap::Parser::parse` and perform live I/O), so providing a real dispatch
/// here would just produce dead-code warnings. Tests exercise the helpers
/// directly.
#[cfg(coverage)]
pub async fn run() -> i32 {
    0
}

/// Parses CLI arguments, routes to the appropriate command handler, and returns
/// the process exit code.
#[cfg(not(coverage))]
pub async fn run() -> i32 {
    let cli = Cli::parse();
    let fields = cli.fields.as_slice();

    match &cli.command {
        Commands::AdhocScreen(args) => commands::adhoc_screen::handle(args, fields).await,
        Commands::Chart(args) => commands::chart::handle(args, fields).await,
        Commands::Fundamentals(args) => commands::fundamentals::handle(args, fields).await,
        Commands::Industry(args) => commands::industry::handle(args, fields).await,
        Commands::MarketData(args) => commands::market_data::handle(args, fields).await,
        Commands::Ownership(args) => commands::ownership::handle(args, fields).await,
        Commands::Ratings(args) => commands::ratings::handle(args, fields).await,
        Commands::Screen(args) => commands::screen::handle(args, fields).await,
        Commands::Tree(args) => commands::tree::handle(args, fields).await,
        Commands::Watchlist(args) => commands::watchlist::handle(args, fields).await,
        Commands::Completions(args) => {
            commands::completions::handle(args);
            0
        }
    }
}
