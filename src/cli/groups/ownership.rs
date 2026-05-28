//! Ownership group: fund ownership summaries and fund holdings.

use clap::Subcommand;

use crate::cli::commands;
use crate::cli::{OwnershipArgs, SymbolLimitArgs};

/// Ownership subcommands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Fetch quarterly fund ownership summary rows for symbols.
    #[command(
        long_about = "Fetch quarterly fund ownership summary rows for symbols. The funds_float_pct_held field is the current percentage of float held by funds from the MarketSurge response. MarketSurge does not provide this value per quarter in the ownership summary payload, so the CLI repeats the current value on each quarterly row for context. Use num_funds_held for historical quarter-by-quarter trend analysis.",
        after_help = "Examples:\n  marketsurge-agent ownership summary AAPL MSFT\n  marketsurge-agent ownership summary --limit 4 AAPL\n\nField notes:\n  funds_float_pct_held is current-only in the MarketSurge response and is repeated on each quarterly row. Use num_funds_held for historical quarter-by-quarter trend analysis."
    )]
    Summary(SymbolLimitArgs),

    /// Fetch individual fund holders and share history for symbols.
    #[command(
        after_help = "Examples:\n  marketsurge-agent ownership funds AAPL MSFT\n  marketsurge-agent ownership funds --limit 10 AAPL"
    )]
    Funds(SymbolLimitArgs),
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
