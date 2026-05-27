use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use super::commands::industry::IndustryCommand;
use super::commands::ownership::OwnershipCommand;
use super::commands::screen::ScreenCommand;
use super::commands::tree::TreeCommand;
use super::commands::watchlist::WatchlistCommand;
use super::groups;

/// CLI tool for querying MarketSurge market data.
#[derive(Debug, Parser)]
#[command(
    name = "marketsurge-agent",
    version,
    about = "Query MarketSurge data as compact JSON",
    long_about = "Query MarketSurge data as compact JSON. Auth reads browser cookies, so log in at https://marketsurge.investors.com first. Use --fields to limit top-level JSON fields in command output.",
    after_help = "Examples:\n  marketsurge-agent analysis ratings AAPL\n  marketsurge-agent --fields symbol,rs_rating analysis ratings AAPL\n  marketsurge-agent completions zsh > _marketsurge-agent",
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
    /// Market data: chart OHLCV bars and broad snapshots.
    #[command(
        after_help = "Examples:\n  marketsurge-agent market chart AAPL MSFT\n  marketsurge-agent market snapshot AAPL MSFT"
    )]
    Market {
        /// Market subcommand to run.
        #[command(subcommand)]
        command: Option<groups::market::Cmd>,
    },

    /// Analysis: company fundamentals and RS ratings.
    #[command(
        after_help = "Examples:\n  marketsurge-agent analysis fundamentals AAPL MSFT\n  marketsurge-agent analysis ratings AAPL MSFT"
    )]
    Analysis {
        /// Analysis subcommand to run.
        #[command(subcommand)]
        command: Option<groups::analysis::Cmd>,
    },

    /// Stock screens: ad-hoc queries, list, and run.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen adhoc --symbols AAPL,MSFT\n  marketsurge-agent screen list --query ibd\n  marketsurge-agent screen run 'IBD 50'"
    )]
    Screen {
        /// Screen subcommand to run.
        #[command(subcommand)]
        command: Option<groups::screen::Cmd>,
    },

    /// Fund ownership data: quarterly summaries and fund holdings.
    #[command(
        after_help = "Examples:\n  marketsurge-agent ownership summary AAPL\n  marketsurge-agent ownership funds AAPL"
    )]
    Ownership {
        /// Ownership subcommand to run.
        #[command(subcommand)]
        command: Option<groups::ownership::Cmd>,
    },

    /// Industry group data: RS ratings and overview.
    #[command(
        after_help = "Examples:\n  marketsurge-agent industry rs AAPL\n  marketsurge-agent industry overview AAPL"
    )]
    Industry {
        /// Industry subcommand to run.
        #[command(subcommand)]
        command: Option<groups::industry::Cmd>,
    },

    /// Navigation trees: coach and site navigation.
    #[command(
        after_help = "Examples:\n  marketsurge-agent navigation coach\n  marketsurge-agent navigation nav"
    )]
    Navigation {
        /// Navigation subcommand to run.
        #[command(subcommand)]
        command: Option<groups::navigation::Cmd>,
    },

    /// Watchlist data: list, symbols, and screening.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist list --query ibd\n  marketsurge-agent watchlist symbols 12345"
    )]
    Watchlist {
        /// Watchlist subcommand to run.
        #[command(subcommand)]
        command: Option<groups::watchlist::Cmd>,
    },

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
#[derive(Debug, Args, Clone)]
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
            "analysis",
            "ratings",
            "AAPL",
        ]);

        assert_eq!(cli.fields, vec!["symbol", "rs_rating"]);
        assert!(matches!(cli.command, Commands::Analysis { .. }));
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
        assert!(matches!(cli.command, Commands::Ownership { .. }));
    }

    #[test]
    fn json_objects_is_not_accepted() {
        let result = Cli::try_parse_from([
            "marketsurge-agent",
            "--json-objects",
            "analysis",
            "ratings",
            "AAPL",
        ]);

        assert!(result.is_err());
    }
}
