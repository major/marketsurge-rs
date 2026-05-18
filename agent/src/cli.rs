use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use crate::commands::adhoc_screen::AdhocScreenCommandArgs;
use crate::commands::industry::IndustryCommand;
use crate::commands::ownership::OwnershipCommand;
use crate::commands::screen::ScreenCommand;
use crate::commands::tree::TreeCommand;
use crate::commands::watchlist::WatchlistCommand;

/// CLI tool for querying MarketSurge market data.
#[derive(Debug, Parser)]
#[command(
    name = "marketsurge-agent",
    version,
    about = "CLI tool for querying MarketSurge market data",
    long_about = "marketsurge-agent queries market data from MarketSurge.\n\n\
        Use it for fund ownership summaries and other market intelligence.\n\n\
        Auth: reads browser cookies automatically. If auth fails with exit code 2,\n\
        log in at https://marketsurge.investors.com in your browser, then retry.\n\n\
        Output: compact JSON to stdout. Pipe through jq for pretty-printing.\n\
        Errors and logs go to stderr.",
    arg_required_else_help = true,
    propagate_version = true
)]
pub struct Cli {
    /// Emit array-of-objects instead of the default array-of-arrays table format.
    #[arg(long, global = true)]
    pub json_objects: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level command groups.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run an ad-hoc screen query against the MarketSurge screener.
    AdhocScreen(AdhocScreenCommandArgs),
    /// Fetch OHLCV chart data (daily by default, weekly with --weekly).
    Chart(ChartArgs),
    /// Fetch fundamental financial data (EPS, sales, estimates).
    Fundamentals(SymbolsArgs),
    /// Industry group data commands (RS rating, overview).
    Industry(IndustryArgs),
    /// Fetch broad market data snapshot (ratings, pricing, industry, fundamentals).
    MarketData(SymbolsArgs),
    /// Fetch user-saved chart markups for a symbol.
    Markups(MarkupsArgs),
    /// Fund ownership data commands.
    Ownership(OwnershipArgs),
    /// Fetch RS rating and relative strength data.
    Ratings(SymbolsArgs),
    /// Stock screen commands (list, run). Supports predefined coach screens by name.
    Screen(ScreenArgs),
    /// Navigation and coaching tree commands (coach, nav).
    Tree(TreeArgs),
    /// Watchlist data commands (list, symbols, screen).
    Watchlist(WatchlistArgs),
    /// Generate shell completions.
    Completions(CompletionsArgs),
}

/// Arguments for the chart command.
#[derive(Debug, Args)]
pub struct ChartArgs {
    /// Fetch weekly data instead of daily.
    #[arg(long)]
    pub weekly: bool,

    /// Ticker symbols and options.
    #[command(flatten)]
    pub symbols: SymbolsArgs,
}

/// Arguments for the industry command group.
#[derive(Debug, Args)]
pub struct IndustryArgs {
    /// Industry subcommand to run.
    #[command(subcommand)]
    pub command: IndustryCommand,
}

/// Arguments for the ownership command group.
#[derive(Debug, Args)]
pub struct OwnershipArgs {
    /// Ownership subcommand to run.
    #[command(subcommand)]
    pub command: OwnershipCommand,
}

/// Arguments for the screen command group.
#[derive(Debug, Args)]
pub struct ScreenArgs {
    /// Screen subcommand to run.
    #[command(subcommand)]
    pub command: ScreenCommand,
}

/// Arguments for the tree command group.
#[derive(Debug, Args)]
pub struct TreeArgs {
    /// Tree subcommand to run.
    #[command(subcommand)]
    pub command: TreeCommand,
}

/// Arguments for the watchlist command group.
#[derive(Debug, Args)]
pub struct WatchlistArgs {
    /// Watchlist subcommand to run.
    #[command(subcommand)]
    pub command: WatchlistCommand,
}

/// Arguments for the markups command.
#[derive(Debug, Args)]
pub struct MarkupsArgs {
    /// Dow Jones key for the symbol (e.g. "13-5320").
    pub dow_jones_key: String,

    /// Filter by frequency (e.g. "DAILY", "WEEKLY").
    #[arg(long)]
    pub frequency: Option<String>,

    /// Sort direction (e.g. "ASC", "DESC").
    #[arg(long)]
    pub sort_dir: Option<String>,
}

/// Arguments for shell completion generation.
#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    pub shell: Shell,
}

/// Arguments containing one or more ticker symbols.
#[derive(Debug, Args)]
pub struct SymbolsArgs {
    /// Ticker symbols to query (e.g. AAPL MSFT).
    #[arg(required = true)]
    pub symbols: Vec<String>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn command_tree_is_valid() {
        Cli::command().debug_assert();
    }
}
