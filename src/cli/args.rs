use chrono::NaiveDate;
use clap::{Args, Parser, Subcommand, ValueEnum};
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
    long_about = "Query MarketSurge data as compact JSON. Auth reads browser cookies, so log in at https://marketsurge.investors.com first. Use --fields to limit top-level JSON fields in command output. Failures emit machine-readable JSON on stderr.",
    after_help = "Examples:\n  marketsurge-agent analysis ratings AAPL\n  marketsurge-agent --fields symbol,rs_rating analysis ratings AAPL\n  marketsurge-agent completions zsh > _marketsurge-agent\n\nDiagnostics:\n  --verbose, -v                  info-level diagnostics to stderr\n  --verbose --verbose, -vv       debug-level diagnostics to stderr\n  --debug                        debug-level diagnostics to stderr\n  RUST_LOG=rusty_marketsurge=debug  env-var equivalent to --debug\n\nExit codes:\n  0  success - command completed successfully\n  1  internal_error - unexpected internal error, including local output failures\n  2  usage - invalid arguments or command usage\n  3  api_error - network failure, rate limit, or upstream MarketSurge API failure\n  4  auth_error - browser cookies are missing, expired, or rejected\n  5  no_results - command completed but produced no actionable result",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Print verbose diagnostics to stderr: once for info, twice for debug.
    ///
    /// Diagnostics include HTTP status codes, auth discovery steps, and
    /// retry decisions.  Cookie values, auth tokens, and full sensitive
    /// headers are never logged.
    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Print detailed debug diagnostics to stderr.
    ///
    /// Equivalent to `--verbose --verbose` or `RUST_LOG=rusty_marketsurge=debug`.
    /// Debug output never contaminates stdout JSON.
    #[arg(long, global = true)]
    pub debug: bool,

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
    /// Combined single-call stock overview for agent consumers.
    #[command(
        after_help = "Examples:\n  marketsurge-agent analyze AAPL\n  marketsurge-agent analyze AAPL MSFT NVDA\n  marketsurge-agent analyze --sections snapshot,ratings,fundamentals AAPL"
    )]
    Analyze(AnalyzeArgs),

    /// Market data: chart OHLCV bars and broad snapshots.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent market chart AAPL MSFT\n  marketsurge-agent market snapshot AAPL MSFT"
    )]
    Market {
        /// Market subcommand to run.
        #[command(subcommand)]
        command: Option<groups::market::Cmd>,
    },

    /// Analysis: company fundamentals and RS ratings.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent analysis fundamentals AAPL MSFT\n  marketsurge-agent analysis ratings AAPL MSFT"
    )]
    Analysis {
        /// Analysis subcommand to run.
        #[command(subcommand)]
        command: Option<groups::analysis::Cmd>,
    },

    /// Stock screens: column discovery, ad-hoc queries, list, and run.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent screen columns\n  marketsurge-agent screen adhoc --symbols AAPL,MSFT\n  marketsurge-agent screen list --query ibd\n  marketsurge-agent screen run 'IBD 50'"
    )]
    Screen {
        /// Screen subcommand to run.
        #[command(subcommand)]
        command: Option<groups::screen::Cmd>,
    },

    /// Fund ownership data: quarterly summaries and fund holdings.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent ownership summary AAPL\n  marketsurge-agent ownership funds AAPL"
    )]
    Ownership {
        /// Ownership subcommand to run.
        #[command(subcommand)]
        command: Option<groups::ownership::Cmd>,
    },

    /// Industry group data: RS ratings and overview.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent industry rs AAPL\n  marketsurge-agent industry overview AAPL"
    )]
    Industry {
        /// Industry subcommand to run.
        #[command(subcommand)]
        command: Option<groups::industry::Cmd>,
    },

    /// Navigation trees: coach and site navigation.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent navigation coach\n  marketsurge-agent navigation nav"
    )]
    Navigation {
        /// Navigation subcommand to run.
        #[command(subcommand)]
        command: Option<groups::navigation::Cmd>,
    },

    /// Watchlist data: list, symbols, and screening.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent watchlist list --query ibd\n  marketsurge-agent watchlist symbols 12345"
    )]
    Watchlist {
        /// Watchlist subcommand to run.
        #[command(subcommand)]
        command: Option<groups::watchlist::Cmd>,
    },

    /// Auth: verify browser cookie and JWT readiness.
    #[command(
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = "Examples:\n  marketsurge-agent auth status"
    )]
    Auth {
        /// Auth subcommand to run.
        #[command(subcommand)]
        command: Option<groups::auth::Cmd>,
    },

    /// Generate shell completion scripts.
    #[command(
        after_help = "Examples:\n  marketsurge-agent completions zsh > _marketsurge-agent\n  marketsurge-agent completions bash > marketsurge-agent.bash"
    )]
    Completions(CompletionsArgs),

    /// Dump the CLI surface as machine-readable JSON.
    #[command(
        long_about = "Dump the CLI surface as machine-readable JSON. The output format is experimental and may change between versions. schema_version 4 documents top-level output fields for field-filterable commands.\n\nThis command does not read browser cookies and does not make any network requests.",
        after_help = "Examples:\n  marketsurge-agent schema\n  marketsurge-agent schema | jq '.commands | length'"
    )]
    Schema,

    /// Run diagnostic checks to verify the tool is working.
    #[command(
        long_about = "Run diagnostic checks to verify the tool is configured correctly. Always writes compact JSON to stdout so scripts and LLM agents can consume the results. Exit codes reflect the worst check result. Network checks (JWT exchange, GraphQL connectivity) are planned but not yet implemented.",
        after_help = "Examples:\n  marketsurge-agent doctor\n  marketsurge-agent doctor --skip-network\n  marketsurge-agent doctor | jq .summary"
    )]
    Doctor(DoctorArgs),
}

/// Sections available in the combined analyze command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AnalyzeSectionArg {
    /// Broad market snapshot fields.
    Snapshot,
    /// Relative strength rating history.
    Ratings,
    /// Fundamental reported and estimated metrics.
    Fundamentals,
    /// Industry relative strength.
    Industry,
    /// Fund ownership summary.
    Ownership,
}

/// Arguments for the combined analyze command.
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// Comma-delimited sections to include. Defaults to all sections.
    #[arg(long, value_delimiter = ',', value_enum, value_name = "SECTION")]
    pub sections: Vec<AnalyzeSectionArg>,

    /// Ticker symbols and options.
    #[command(flatten)]
    pub symbols: SymbolsArgs,
}

/// Arguments for the chart command.
#[derive(Debug, Args)]
pub struct ChartArgs {
    /// Use weekly bars instead of daily bars.
    #[arg(long)]
    pub weekly: bool,

    /// Keep only the last N returned bars per symbol.
    #[arg(long, value_name = "COUNT", value_parser = clap::value_parser!(u16).range(1..))]
    pub days: Option<u16>,

    /// Start chart history at this date, formatted as YYYY-MM-DD.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub start_date: Option<NaiveDate>,

    /// Keep only the first N output rows. Use 0 for no limit.
    #[command(flatten)]
    pub limit: LimitArgs,

    /// Ticker symbols and options.
    #[command(flatten)]
    pub symbols: SymbolsArgs,
}

/// Arguments for limiting data-returning command output.
#[derive(Debug, Args, Clone, Copy, Default)]
pub struct LimitArgs {
    /// Keep only the first N output rows. Use 0 for no limit.
    #[arg(long, default_value_t = 0, value_name = "COUNT")]
    pub limit: usize,
}

/// Arguments containing ticker symbols plus output row limiting.
#[derive(Debug, Args, Clone)]
pub struct SymbolLimitArgs {
    /// Output row limit.
    #[command(flatten)]
    pub limit: LimitArgs,

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

/// Arguments for the doctor diagnostic command.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Skip network-based checks (JWT exchange, GraphQL connectivity).
    #[arg(long)]
    pub skip_network: bool,
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
    fn analyze_accepts_default_sections() {
        let cli = Cli::parse_from(["marketsurge-agent", "analyze", "AAPL"]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("Analyze"));
        assert!(command.contains("sections: []"));
        assert!(command.contains("symbols: [\"AAPL\"]"));
    }

    #[test]
    fn analyze_accepts_selected_sections_and_multiple_symbols() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "analyze",
            "--sections",
            "snapshot,ratings,fundamentals",
            "AAPL",
            "MSFT",
        ]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("Snapshot"));
        assert!(command.contains("Ratings"));
        assert!(command.contains("Fundamentals"));
        assert!(command.contains("symbols: [\"AAPL\", \"MSFT\"]"));
    }

    #[test]
    fn analyze_rejects_unknown_sections() {
        let result = Cli::try_parse_from([
            "marketsurge-agent",
            "analyze",
            "--sections",
            "snapshot,unknown",
            "AAPL",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn market_chart_accepts_date_range_flags() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "market",
            "chart",
            "--weekly",
            "--days",
            "20",
            "--limit",
            "5",
            "--start-date",
            "2026-05-01",
            "AAPL",
        ]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("weekly: true"));
        assert!(command.contains("days: Some(20)"));
        assert!(command.contains("limit: 5"));
        assert!(command.contains("start_date: Some(2026-05-01)"));
        assert!(command.contains("symbols: [\"AAPL\"]"));
    }

    #[test]
    fn data_commands_accept_zero_limit() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "analysis",
            "fundamentals",
            "--limit",
            "0",
            "AAPL",
        ]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("limit: 0"));
        assert!(command.contains("symbols: [\"AAPL\"]"));
    }

    #[test]
    fn data_commands_accept_positive_limit() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "ownership",
            "funds",
            "--limit",
            "10",
            "AAPL",
        ]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("limit: 10"));
        assert!(command.contains("symbols: [\"AAPL\"]"));
    }

    #[test]
    fn data_commands_reject_negative_limit() {
        let result = Cli::try_parse_from([
            "marketsurge-agent",
            "industry",
            "rs",
            "--limit",
            "-1",
            "AAPL",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn market_chart_rejects_zero_days() {
        let result = Cli::try_parse_from([
            "marketsurge-agent",
            "market",
            "chart",
            "--days",
            "0",
            "AAPL",
        ]);

        assert!(result.is_err());
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
