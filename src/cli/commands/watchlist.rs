//! Watchlist data commands.

use crate::watchlist::{WatchlistDetail, WatchlistSummary};
use clap::{Args, Subcommand};
use serde::Serialize;
use tracing::instrument;

use crate::cli::WatchlistArgs;
use crate::cli::common::command::{api_call, run_client_command, run_command};
use crate::cli::common::rows::{flatten_response_rows, response_columns};

/// Watchlist subcommands.
#[derive(Debug, Subcommand)]
pub enum WatchlistCommand {
    /// List saved watchlists.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist list\n  marketsurge-agent watchlist list --query ibd"
    )]
    List(WatchlistListArgs),
    /// Fetch symbols in a watchlist by ID.
    #[command(after_help = "Examples:\n  marketsurge-agent watchlist symbols 12345")]
    Symbols(WatchlistSymbolsArgs),
    /// Screen symbols with selected MarketSurge data columns.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist screen AAPL MSFT\n  marketsurge-agent watchlist screen AAPL --columns Symbol,EPSRating,RSRating"
    )]
    Screen(WatchlistScreenArgs),
}

/// Arguments for the watchlist list subcommand.
#[derive(Debug, Args)]
pub struct WatchlistListArgs {
    /// Filter watchlists by ID, name, or description.
    #[arg(long, short)]
    pub query: Option<String>,
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
        WatchlistCommand::List(a) => execute_list(a, fields).await,
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

fn filter_watchlist_list(
    records: Vec<WatchlistRecord>,
    normalized_query: Option<&str>,
) -> Vec<WatchlistRecord> {
    let Some(normalized_query) = normalized_query else {
        return records;
    };

    records
        .into_iter()
        .filter(|record| watchlist_record_matches(record, normalized_query))
        .collect()
}

fn normalized_watchlist_query(query: Option<&str>) -> Option<String> {
    query
        .map(normalized_watchlist_name)
        .filter(|query| !query.is_empty())
}

fn watchlist_record_matches(record: &WatchlistRecord, normalized_query: &str) -> bool {
    [
        record.id.as_deref(),
        record.name.as_deref(),
        record.description.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| normalized_watchlist_name(value).contains(normalized_query))
}

fn normalized_watchlist_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_list(args: &WatchlistListArgs, fields: &[String]) -> i32 {
    let query = normalized_watchlist_query(args.query.as_deref());

    run_client_command(fields, |client| async move {
        let response = api_call(client.get_all_watchlist_names()).await?;

        Ok(filter_watchlist_list(
            flatten_watchlist_list(&response.watchlists),
            query.as_deref(),
        ))
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
    use crate::cli::common::test_support::{response_value, response_value_without_md_item};
    use crate::watchlist::WatchlistItem;

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
    fn filter_list_matches_name_without_punctuation() {
        let records = flatten_watchlist_list(&[
            WatchlistSummary {
                id: Some("1".into()),
                name: Some("EF-50".into()),
                last_modified_date_utc: None,
                description: None,
            },
            WatchlistSummary {
                id: Some("2".into()),
                name: Some("IBD 50".into()),
                last_modified_date_utc: None,
                description: None,
            },
        ]);

        let filtered = filter_watchlist_list(records, Some("ibd50"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id.as_deref(), Some("2"));
    }

    #[test]
    fn filter_list_matches_description() {
        let records = flatten_watchlist_list(&[WatchlistSummary {
            id: Some("1".into()),
            name: Some("Growth".into()),
            last_modified_date_utc: None,
            description: Some("IBD leaders".into()),
        }]);

        let filtered = filter_watchlist_list(records, Some("ibd"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id.as_deref(), Some("1"));
    }

    #[test]
    fn filter_list_matches_id() {
        let records = flatten_watchlist_list(&[WatchlistSummary {
            id: Some("watchlist-ibd-50".into()),
            name: Some("Growth".into()),
            last_modified_date_utc: None,
            description: None,
        }]);

        let filtered = filter_watchlist_list(records, Some("ibd50"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id.as_deref(), Some("watchlist-ibd-50"));
    }

    #[test]
    fn filter_list_without_query_returns_all_records() {
        let records = flatten_watchlist_list(&[WatchlistSummary {
            id: Some("1".into()),
            name: Some("Growth".into()),
            last_modified_date_utc: None,
            description: None,
        }]);

        let filtered = filter_watchlist_list(records, None);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn normalized_watchlist_query_ignores_empty_normalized_query() {
        assert_eq!(normalized_watchlist_query(Some(" -- ")), None);
        assert_eq!(
            normalized_watchlist_query(Some("IBD 50")).as_deref(),
            Some("ibd50")
        );
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
