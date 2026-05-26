use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use super::commands::adhoc_screen::AdhocScreenCommandArgs;
use super::commands::industry::IndustryCommand;
use super::commands::ownership::OwnershipCommand;
use super::commands::screen::ScreenCommand;
use super::commands::tree::TreeCommand;
use super::commands::watchlist::WatchlistCommand;

/// CLI tool for querying MarketSurge market data.
#[derive(Debug, Parser)]
#[command(
    name = "marketsurge-agent",
    version,
    about = "Query MarketSurge data as compact JSON",
    long_about = "Query MarketSurge data as compact JSON. Auth reads browser cookies, so log in at https://marketsurge.investors.com first. Use --fields to limit top-level JSON fields in command output.",
    after_help = "Examples:\n  marketsurge-agent ratings AAPL MSFT\n  marketsurge-agent --fields symbol,rs_rating ratings AAPL\n  marketsurge-agent completions zsh > _marketsurge-agent",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Comma-delimited top-level JSON fields to include in output.
    #[arg(long, global = true, value_delimiter = ',', value_name = "FIELD")]
    pub fields: Vec<String>,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level command groups.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run an ad-hoc screener query and return matching rows.
    #[command(
        after_help = "Examples:\n  marketsurge-agent adhoc-screen --symbols AAPL,MSFT --columns Symbol,CompanyName,EPSRating\n  marketsurge-agent adhoc-screen --screen-id 12345 --limit 100"
    )]
    AdhocScreen(AdhocScreenCommandArgs),
    /// Fetch daily or weekly OHLCV bars for symbols.
    #[command(
        after_help = "Examples:\n  marketsurge-agent chart AAPL MSFT\n  marketsurge-agent chart --weekly AAPL"
    )]
    Chart(ChartArgs),
    /// Fetch EPS, sales, and estimate fundamentals for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent fundamentals AAPL MSFT")]
    Fundamentals(SymbolsArgs),
    /// Fetch industry group RS and overview data.
    #[command(
        after_help = "Examples:\n  marketsurge-agent industry rs AAPL\n  marketsurge-agent industry overview AAPL"
    )]
    Industry(IndustryArgs),
    /// Fetch broad rating, price, industry, and fundamental snapshot data.
    #[command(after_help = "Examples:\n  marketsurge-agent market-data AAPL MSFT")]
    MarketData(SymbolsArgs),
    /// Fetch fund ownership summaries and fund holdings.
    #[command(
        after_help = "Examples:\n  marketsurge-agent ownership summary AAPL\n  marketsurge-agent ownership funds AAPL"
    )]
    Ownership(OwnershipArgs),
    /// Fetch relative strength ratings for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent ratings AAPL MSFT")]
    Ratings(SymbolsArgs),
    /// List or run stock screens, including coach screens.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen list --query ibd\n  marketsurge-agent screen list --coach\n  marketsurge-agent screen run 'IBD 50'"
    )]
    Screen(ScreenArgs),
    /// Fetch coach or navigation trees.
    #[command(
        after_help = "Examples:\n  marketsurge-agent tree coach\n  marketsurge-agent tree nav"
    )]
    Tree(TreeArgs),
    /// List watchlists, read symbols, or screen symbols.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist list --query ibd\n  marketsurge-agent watchlist symbols 12345"
    )]
    Watchlist(WatchlistArgs),
    /// Generate shell completion scripts.
    #[command(
        after_help = "Examples:\n  marketsurge-agent completions zsh > _marketsurge-agent\n  marketsurge-agent completions bash > marketsurge-agent.bash"
    )]
    Completions(CompletionsArgs),
    /// Dump the CLI surface as machine-readable JSON.
    #[command(
        long_about = "Dump the CLI surface as machine-readable JSON. The output format is experimental and may change between versions. schema_version 1 is the initial format.\n\nThis command does not read browser cookies and does not make any network requests.",
        after_help = "Examples:\n  marketsurge-agent schema\n  marketsurge-agent schema | jq '.commands | length'"
    )]
    Schema,
}

/// Arguments for the chart command.
#[derive(Debug, Args)]
pub struct ChartArgs {
    /// Use weekly bars instead of daily bars.
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

/// Arguments for shell completion generation.
#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Target shell for completion output.
    pub shell: Shell,
}

/// Arguments containing one or more ticker symbols.
#[derive(Debug, Args)]
pub struct SymbolsArgs {
    /// One or more ticker symbols, for example AAPL MSFT.
    #[arg(required = true)]
    pub symbols: Vec<String>,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Commands};

    #[test]
    fn command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn fields_can_precede_subcommands() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "--fields",
            "symbol,rs_rating",
            "ratings",
            "AAPL",
        ]);

        assert_eq!(cli.fields, vec!["symbol", "rs_rating"]);
        assert!(matches!(cli.command, Commands::Ratings(_)));
    }

    #[test]
    fn fields_can_follow_nested_subcommands() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "ownership",
            "summary",
            "--fields",
            "symbol,num_funds_held",
            "AAPL",
        ]);

        assert_eq!(cli.fields, vec!["symbol", "num_funds_held"]);
        assert!(matches!(cli.command, Commands::Ownership(_)));
    }

    #[test]
    fn json_objects_is_not_accepted() {
        let result =
            Cli::try_parse_from(["marketsurge-agent", "--json-objects", "ratings", "AAPL"]);

        assert!(result.is_err());
    }
}
