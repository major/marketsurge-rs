//! Watchlist data commands.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use marketsurge_client::screen::ResponseValue;
use marketsurge_client::types::ResponseColumn;
use marketsurge_client::watchlist::{WatchlistDetail, WatchlistSummary};
use serde::Serialize;
use tracing::instrument;

use crate::cli::WatchlistArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_client_command, run_command};

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
pub async fn handle(args: &WatchlistArgs, json_table: bool) -> i32 {
    match &args.command {
        WatchlistCommand::List => execute_list(json_table).await,
        WatchlistCommand::Symbols(a) => execute_symbols(a, json_table).await,
        WatchlistCommand::Screen(a) => execute_screen(a, json_table).await,
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
async fn execute_list(json_table: bool) -> i32 {
    run_client_command(json_table, |client| async move {
        let response = client
            .get_all_watchlist_names()
            .await
            .map_err(handle_api_error)?;

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
async fn execute_symbols(args: &WatchlistSymbolsArgs, json_table: bool) -> i32 {
    let watchlist_id = args.watchlist_id.clone();

    run_client_command(json_table, |client| async move {
        let response = client
            .flagged_symbols(&watchlist_id)
            .await
            .map_err(handle_api_error)?;

        Ok(flatten_watchlist_symbols(response.watchlist.as_ref()))
    })
    .await
}

/// Converts screen response rows into flat key-value maps.
///
/// Each row becomes a `BTreeMap` mapping column name to cell value. Cells
/// without a named `md_item` are skipped.
fn flatten_watchlist_screen(
    response_values: &[Vec<ResponseValue>],
) -> Vec<BTreeMap<String, Option<String>>> {
    response_values
        .iter()
        .map(|row| {
            row.iter()
                .filter_map(|cell| {
                    let name = cell.md_item.as_ref().and_then(|m| m.name.clone())?;
                    Some((name, cell.value.clone()))
                })
                .collect()
        })
        .collect()
}

#[instrument(skip_all)]
async fn execute_screen(args: &WatchlistScreenArgs, json_table: bool) -> i32 {
    let columns: Vec<ResponseColumn> = args
        .columns
        .iter()
        .map(|name| ResponseColumn {
            name: name.clone(),
            sort_information: None,
        })
        .collect();

    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .screener_watchlist_items(&symbol_refs, columns)
                .await
                .map_err(handle_api_error)?;

            let empty = Vec::new();
            let rows = response
                .market_data_adhoc_screen
                .as_ref()
                .map(|result| &result.response_values)
                .unwrap_or(&empty);

            Ok(flatten_watchlist_screen(rows))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use marketsurge_client::screen::MdItem;
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
            ResponseValue {
                value: Some("95".into()),
                md_item: Some(MdItem {
                    md_item_id: None,
                    name: Some("EPSRating".into()),
                }),
            },
            ResponseValue {
                value: Some("88".into()),
                md_item: Some(MdItem {
                    md_item_id: None,
                    name: Some("RSRating".into()),
                }),
            },
        ]];

        let records = flatten_watchlist_screen(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("EPSRating"), Some(&Some("95".into())));
        assert_eq!(records[0].get("RSRating"), Some(&Some("88".into())));
    }

    #[test]
    fn flatten_screen_empty_rows() {
        let records = flatten_watchlist_screen(&[]);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_screen_skips_missing_md_item() {
        let rows = vec![vec![
            ResponseValue {
                value: Some("99".into()),
                md_item: None,
            },
            ResponseValue {
                value: Some("A".into()),
                md_item: Some(MdItem {
                    md_item_id: None,
                    name: Some("SMRRating".into()),
                }),
            },
        ]];

        let records = flatten_watchlist_screen(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].len(), 1);
        assert_eq!(records[0].get("SMRRating"), Some(&Some("A".into())));
    }

    #[test]
    fn flatten_screen_none_value_preserved() {
        let rows = vec![vec![ResponseValue {
            value: None,
            md_item: Some(MdItem {
                md_item_id: None,
                name: Some("CompRating".into()),
            }),
        }]];

        let records = flatten_watchlist_screen(&rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("CompRating"), Some(&None));
    }
}
