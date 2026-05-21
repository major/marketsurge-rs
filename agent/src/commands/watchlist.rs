//! Watchlist data commands.

use clap::{Args, Subcommand};
use marketsurge_client::watchlist::{WatchlistDetail, WatchlistSummary};
use serde::Serialize;
use tracing::instrument;

use crate::cli::WatchlistArgs;
use crate::common::command::{api_call, run_client_command, run_command};
use crate::common::rows::{flatten_response_rows, response_columns};

/// Watchlist subcommands.
#[derive(Debug, Subcommand)]
pub enum WatchlistCommand {
    /// List saved watchlists.
    #[command(after_help = "Examples:\n  marketsurge-agent watchlist list")]
    List,
    /// Fetch symbols in a watchlist by ID.
    #[command(after_help = "Examples:\n  marketsurge-agent watchlist symbols 12345")]
    Symbols(WatchlistSymbolsArgs),
    /// Screen symbols with selected MarketSurge data columns.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist screen AAPL MSFT\n  marketsurge-agent watchlist screen AAPL --columns Symbol,EPSRating,RSRating"
    )]
    Screen(WatchlistScreenArgs),
}

/// Arguments for the watchlist symbols subcommand.
#[derive(Debug, Args)]
pub struct WatchlistSymbolsArgs {
    /// Watchlist ID from `watchlist list`.
    pub watchlist_id: String,
}

/// Arguments for the watchlist screen subcommand.
#[derive(Debug, Args)]
pub struct WatchlistScreenArgs {
    /// Symbols to screen, for example AAPL MSFT.
    #[arg(required = true)]
    pub symbols: Vec<String>,

    /// Output columns, comma-separated.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "EPSRating,RSRating,AccDisRating,CompRating,SMRRating"
    )]
    pub columns: Vec<String>,
}

/// Flat output record for a watchlist listing entry.
#[derive(Debug, Clone, Serialize)]
pub struct WatchlistRecord {
    /// Watchlist identifier.
    pub id: Option<String>,
    /// Watchlist name.
    pub name: Option<String>,
    /// Last modified timestamp in UTC.
    pub last_modified: Option<String>,
    /// Watchlist description.
    pub description: Option<String>,
}

/// Flat output record for a watchlist symbol.
#[derive(Debug, Clone, Serialize)]
pub struct WatchlistSymbolRecord {
    /// Watchlist identifier.
    pub watchlist_id: Option<String>,
    /// Watchlist name.
    pub watchlist_name: Option<String>,
    /// Symbol key (e.g. "AAPL").
    pub key: Option<String>,
    /// Dow Jones symbol key (e.g. "US:AAPL").
    pub dow_jones_key: Option<String>,
}

/// Handles the watchlist command group.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &WatchlistArgs, fields: &[String]) -> i32 {
    match &args.command {
        WatchlistCommand::List => execute_list(fields).await,
        WatchlistCommand::Symbols(a) => execute_symbols(a, fields).await,
        WatchlistCommand::Screen(a) => execute_screen(a, fields).await,
    }
}

/// Converts watchlist summaries into flat output records.
fn flatten_watchlist_list(watchlists: &[WatchlistSummary]) -> Vec<WatchlistRecord> {
    watchlists
        .iter()
        .map(|wl| WatchlistRecord {
            id: wl.id.clone(),
            name: wl.name.clone(),
            last_modified: wl.last_modified_date_utc.clone(),
            description: wl.description.clone(),
        })
        .collect()
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_list(fields: &[String]) -> i32 {
    run_client_command(fields, |client| async move {
        let response = api_call(client.get_all_watchlist_names()).await?;

        Ok(flatten_watchlist_list(&response.watchlists))
    })
    .await
}

/// Extracts symbol records from an optional watchlist detail.
fn flatten_watchlist_symbols(watchlist: Option<&WatchlistDetail>) -> Vec<WatchlistSymbolRecord> {
    watchlist
        .map(|wl| {
            wl.items
                .iter()
                .map(|item| WatchlistSymbolRecord {
                    watchlist_id: wl.id.clone(),
                    watchlist_name: wl.name.clone(),
                    key: item.key.clone(),
                    dow_jones_key: item.dow_jones_key.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_symbols(args: &WatchlistSymbolsArgs, fields: &[String]) -> i32 {
    let watchlist_id = args.watchlist_id.clone();

    run_client_command(fields, |client| async move {
        let response = api_call(client.flagged_symbols(&watchlist_id)).await?;

        Ok(flatten_watchlist_symbols(response.watchlist.as_ref()))
    })
    .await
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_screen(args: &WatchlistScreenArgs, fields: &[String]) -> i32 {
    let columns = response_columns(&args.columns);

    run_command(&args.symbols, fields, |client, symbol_refs| async move {
        let response = api_call(client.screener_watchlist_items(&symbol_refs, columns)).await?;

        let empty = Vec::new();
        let rows = response
            .market_data_adhoc_screen
            .as_ref()
            .map(|result| &result.response_values)
            .unwrap_or(&empty);

        Ok(flatten_response_rows(rows))
    })
    .await
}

#[cfg(test)]
mod tests {
    use crate::common::test_support::{response_value, response_value_without_md_item};
    use marketsurge_client::watchlist::WatchlistItem;

    use super::*;

    #[test]
    fn flatten_list_maps_fields() {
        let summaries = vec![WatchlistSummary {
            id: Some("1".into()),
            name: Some("Growth".into()),
            last_modified_date_utc: Some("2025-01-01T00:00:00Z".into()),
            description: Some("Top picks".into()),
        }];

        let records = flatten_watchlist_list(&summaries);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id.as_deref(), Some("1"));
        assert_eq!(records[0].name.as_deref(), Some("Growth"));
        assert_eq!(
            records[0].last_modified.as_deref(),
            Some("2025-01-01T00:00:00Z")
        );
        assert_eq!(records[0].description.as_deref(), Some("Top picks"));
    }

    #[test]
    fn flatten_list_empty() {
        let records = flatten_watchlist_list(&[]);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_symbols_maps_fields() {
        let detail = WatchlistDetail {
            id: Some("42".into()),
            name: Some("Tech".into()),
            last_modified_date_utc: None,
            description: None,
            items: vec![
                WatchlistItem {
                    key: Some("AAPL".into()),
                    dow_jones_key: Some("US:AAPL".into()),
                },
                WatchlistItem {
                    key: Some("MSFT".into()),
                    dow_jones_key: Some("US:MSFT".into()),
                },
            ],
        };

        let records = flatten_watchlist_symbols(Some(&detail));

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].watchlist_id.as_deref(), Some("42"));
        assert_eq!(records[0].watchlist_name.as_deref(), Some("Tech"));
        assert_eq!(records[0].key.as_deref(), Some("AAPL"));
        assert_eq!(records[1].key.as_deref(), Some("MSFT"));
        assert_eq!(records[1].dow_jones_key.as_deref(), Some("US:MSFT"));
    }

    #[test]
    fn flatten_symbols_none_returns_empty() {
        let records = flatten_watchlist_symbols(None);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_screen_maps_named_cells() {
        let rows = vec![vec![
            response_value("EPSRating", Some("95")),
            response_value("RSRating", Some("88")),
        ]];

        let records = flatten_response_rows(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("EPSRating"), Some(&Some("95".into())));
        assert_eq!(records[0].get("RSRating"), Some(&Some("88".into())));
    }

    #[test]
    fn flatten_screen_empty_rows() {
        let records = flatten_response_rows(&[]);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_screen_skips_missing_md_item() {
        let rows = vec![vec![
            response_value_without_md_item(Some("99")),
            response_value("SMRRating", Some("A")),
        ]];

        let records = flatten_response_rows(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].len(), 1);
        assert_eq!(records[0].get("SMRRating"), Some(&Some("A".into())));
    }

    #[test]
    fn flatten_screen_none_value_preserved() {
        let rows = vec![vec![response_value("CompRating", None)]];

        let records = flatten_response_rows(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("CompRating"), Some(&None));
    }
}
