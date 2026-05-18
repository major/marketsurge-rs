//! Watchlist data commands.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use marketsurge_client::types::ResponseColumn;
use serde::Serialize;
use tracing::instrument;

use crate::cli::WatchlistArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_client_command, run_command};

/// Watchlist subcommands.
#[derive(Debug, Subcommand)]
pub enum WatchlistCommand {
    /// List all saved watchlists.
    List,
    /// Fetch symbols in a watchlist by ID.
    Symbols(WatchlistSymbolsArgs),
    /// Screen watchlist symbols with specified data columns.
    Screen(WatchlistScreenArgs),
}

/// Arguments for the watchlist symbols subcommand.
#[derive(Debug, Args)]
pub struct WatchlistSymbolsArgs {
    /// Watchlist ID to fetch symbols from.
    pub watchlist_id: String,
}

/// Arguments for the watchlist screen subcommand.
#[derive(Debug, Args)]
pub struct WatchlistScreenArgs {
    /// Ticker symbols to screen (e.g. AAPL MSFT).
    #[arg(required = true)]
    pub symbols: Vec<String>,

    /// Data columns to include (e.g. EPSRating,RSRating,AccDisRating).
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
pub async fn handle(args: &WatchlistArgs, json_table: bool) -> i32 {
    match &args.command {
        WatchlistCommand::List => execute_list(json_table).await,
        WatchlistCommand::Symbols(a) => execute_symbols(a, json_table).await,
        WatchlistCommand::Screen(a) => execute_screen(a, json_table).await,
    }
}

#[instrument(skip_all)]
async fn execute_list(json_table: bool) -> i32 {
    run_client_command(json_table, |client| async move {
        let response = client
            .get_all_watchlist_names()
            .await
            .map_err(handle_api_error)?;

        let records: Vec<WatchlistRecord> = response
            .watchlists
            .iter()
            .map(|wl| WatchlistRecord {
                id: wl.id.clone(),
                name: wl.name.clone(),
                last_modified: wl.last_modified_date_utc.clone(),
                description: wl.description.clone(),
            })
            .collect();

        Ok(records)
    })
    .await
}

#[instrument(skip_all)]
async fn execute_symbols(args: &WatchlistSymbolsArgs, json_table: bool) -> i32 {
    let watchlist_id = args.watchlist_id.clone();

    run_client_command(json_table, |client| async move {
        let response = client
            .flagged_symbols(&watchlist_id)
            .await
            .map_err(handle_api_error)?;

        let records: Vec<WatchlistSymbolRecord> = response
            .watchlist
            .as_ref()
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
            .unwrap_or_default();

        Ok(records)
    })
    .await
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

            let records: Vec<BTreeMap<String, Option<String>>> = response
                .market_data_adhoc_screen
                .as_ref()
                .map(|result| &result.response_values)
                .unwrap_or(&Vec::new())
                .iter()
                .map(|row| {
                    row.iter()
                        .filter_map(|cell| {
                            let name = cell.md_item.as_ref().and_then(|m| m.name.clone())?;
                            Some((name, cell.value.clone()))
                        })
                        .collect()
                })
                .collect();

            Ok(records)
        },
    )
    .await
}
