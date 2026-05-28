//! Command line parsing, dispatch, and output rendering.

mod args;
/// Command handlers for each CLI subcommand group.
pub mod commands;
/// Shared utilities: auth helpers and error mapping.
pub mod common;
/// Command groups that organize related subcommands.
pub mod groups;
/// Tracing subscriber initialization for CLI diagnostics.
pub mod logging;
/// JSON output formatting and field selection.
pub mod output;

pub use args::*;

use clap::Parser;

use common::exit::ExitCode;

/// No-op stub so the binary compiles under `cargo-llvm-cov`.
///
/// Coverage instrumentation excludes the real `handle` functions (which call
/// `clap::Parser::parse` and perform live I/O), so providing a real dispatch
/// here would just produce dead-code warnings. Tests exercise the helpers
/// directly.
#[cfg(coverage)]
pub async fn run() -> i32 {
    ExitCode::Success.code()
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
    ExitCode::Success.code()
}

/// Parses CLI arguments, routes to the appropriate command handler, and returns
/// the process exit code.
#[cfg(not(coverage))]
pub async fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if err.exit_code() != ExitCode::Success.code() {
                return common::error::render_usage_error(&err);
            }

            let _ = err.print();
            return err.exit_code();
        }
    };
    common::error::set_command_name(command_name(&cli.command));

    logging::init(cli.verbose, cli.debug);

    let fields = cli.fields.as_slice();

    match &cli.command {
        Commands::Analyze(args) => commands::analyze::handle(args, fields).await,
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
        Commands::Auth { command } => match command {
            Some(cmd) => groups::auth::dispatch(cmd, fields).await,
            None => print_subcommand_help("auth"),
        },
        Commands::Completions(args) => {
            commands::completions::handle(args);
            ExitCode::Success.code()
        }
        Commands::Schema => commands::schema::handle(fields),
        Commands::Doctor(args) => commands::doctor::handle(fields, args.skip_network),
    }
}

#[cfg(not(coverage))]
fn command_name(command: &Commands) -> Option<&'static str> {
    match command {
        Commands::Analyze(_) => Some("analyze"),
        Commands::Market { .. } => Some("market"),
        Commands::Analysis { .. } => Some("analysis"),
        Commands::Screen { .. } => Some("screen"),
        Commands::Ownership { .. } => Some("ownership"),
        Commands::Industry { .. } => Some("industry"),
        Commands::Navigation { .. } => Some("navigation"),
        Commands::Watchlist { .. } => Some("watchlist"),
        Commands::Auth { .. } => Some("auth"),
        Commands::Completions(_) => Some("completions"),
        Commands::Schema => Some("schema"),
        Commands::Doctor(_) => Some("doctor"),
    }
}
