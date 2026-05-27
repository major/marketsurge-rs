//! Command line parsing, dispatch, and output rendering.

mod args;
/// Command handlers for each CLI subcommand group.
pub mod commands;
/// Shared utilities: auth helpers and error mapping.
pub mod common;
/// Command groups that organize related subcommands.
pub mod groups;
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

/// Prints help for a command group when invoked without a subcommand.
#[cfg(not(coverage))]
fn print_subcommand_help(name: &str) -> i32 {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    cmd.build();
    if let Some(sub) = cmd.find_subcommand(name) {
        // print_help requires &mut, so clone the subcommand to get a mutable copy.
        let mut sub = sub.clone();
        let _ = sub.print_help();
    }
    0
}

/// Parses CLI arguments, routes to the appropriate command handler, and returns
/// the process exit code.
#[cfg(not(coverage))]
pub async fn run() -> i32 {
    let cli = Cli::parse();
    let fields = cli.fields.as_slice();

    match &cli.command {
        Commands::Market { command } => match command {
            Some(cmd) => groups::market::dispatch(cmd, fields).await,
            None => print_subcommand_help("market"),
        },
        Commands::Analysis { command } => match command {
            Some(cmd) => groups::analysis::dispatch(cmd, fields).await,
            None => print_subcommand_help("analysis"),
        },
        Commands::Screen { command } => match command {
            Some(cmd) => groups::screen::dispatch(cmd, fields).await,
            None => print_subcommand_help("screen"),
        },
        Commands::Ownership { command } => match command {
            Some(cmd) => groups::ownership::dispatch(cmd, fields).await,
            None => print_subcommand_help("ownership"),
        },
        Commands::Industry { command } => match command {
            Some(cmd) => groups::industry::dispatch(cmd, fields).await,
            None => print_subcommand_help("industry"),
        },
        Commands::Navigation { command } => match command {
            Some(cmd) => groups::navigation::dispatch(cmd, fields).await,
            None => print_subcommand_help("navigation"),
        },
        Commands::Watchlist { command } => match command {
            Some(cmd) => groups::watchlist::dispatch(cmd, fields).await,
            None => print_subcommand_help("watchlist"),
        },
        Commands::Completions(args) => {
            commands::completions::handle(args);
            0
        }
        Commands::Schema => commands::schema::handle(fields),
    }
}
