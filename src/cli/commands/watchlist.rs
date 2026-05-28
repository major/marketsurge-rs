//! Watchlist data commands.

use crate::watchlist::{WatchlistDetail, WatchlistSummary};
use clap::{Args, Subcommand};
use serde::Serialize;
use tracing::instrument;

use crate::cli::WatchlistArgs;
use crate::cli::common::command::{api_call, run_client_command};
use crate::cli::common::error::{render_no_results_message, render_usage_message_with_suggestion};
use crate::cli::common::rows::{flatten_response_rows, response_columns};

/// Watchlist subcommands.
#[derive(Debug, Subcommand)]
pub enum WatchlistCommand {
    /// List saved watchlists.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist list\n  marketsurge-agent watchlist list --query ibd"
    )]
    List(WatchlistListArgs),
    /// Fetch symbols in a watchlist by ID or exact name.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist symbols 12345\n  marketsurge-agent watchlist symbols \"My Watchlist\""
    )]
    Symbols(WatchlistSymbolsArgs),
    /// Screen symbols or watchlist symbols with selected MarketSurge data columns.
    #[command(
        after_help = "Examples:\n  marketsurge-agent watchlist screen AAPL MSFT\n  marketsurge-agent watchlist screen --symbols \"My Watchlist\"\n  marketsurge-agent watchlist screen AAPL --columns Symbol,EPSRating,RSRating"
    )]
    Screen(WatchlistScreenArgs),
}

/// Arguments for the watchlist list subcommand.
#[derive(Debug, Args, Clone)]
pub struct WatchlistListArgs {
    /// Filter watchlists by ID, name, or description.
    #[arg(long, short)]
    pub query: Option<String>,
}

/// Arguments for the watchlist symbols subcommand.
#[derive(Debug, Args, Clone)]
pub struct WatchlistSymbolsArgs {
    /// Watchlist ID or exact name from `watchlist list`.
    pub watchlist_id: String,
}

/// Arguments for the watchlist screen subcommand.
#[derive(Debug, Args, Clone)]
pub struct WatchlistScreenArgs {
    /// Symbols to screen, for example AAPL MSFT.
    #[arg(
        value_name = "SYMBOL",
        required_unless_present = "watchlist_symbols",
        conflicts_with = "watchlist_symbols"
    )]
    pub symbols: Vec<String>,

    /// Watchlist ID or exact name from `watchlist list` to screen.
    #[arg(long = "symbols", value_name = "WATCHLIST", id = "watchlist_symbols")]
    pub watchlist_symbols: Option<String>,

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
    /// Human-readable ticker symbol.
    pub symbol: Option<String>,
    /// Symbol key (e.g. "AAPL").
    pub key: Option<String>,
    /// Dow Jones symbol key (e.g. "US:AAPL").
    pub dow_jones_key: Option<String>,
}

#[derive(Debug)]
enum WatchlistResolutionError {
    NotFound { query: String },
    Empty { query: String },
    MultipleMatches { query: String, matches: Vec<String> },
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

fn resolve_watchlist_id_from_response(
    response: &crate::watchlist::WatchlistNamesResponse,
    id_or_name: &str,
) -> Result<String, WatchlistResolutionError> {
    if response
        .watchlists
        .iter()
        .any(|watchlist| watchlist.id.as_deref() == Some(id_or_name))
    {
        return Ok(id_or_name.to_string());
    }

    let normalized_target = normalized_watchlist_name(id_or_name);
    let matches: Vec<&WatchlistSummary> = response
        .watchlists
        .iter()
        .filter(|watchlist| {
            watchlist
                .name
                .as_deref()
                .is_some_and(|name| normalized_watchlist_name(name) == normalized_target)
        })
        .collect();

    match matches.as_slice() {
        [watchlist] => watchlist
            .id
            .clone()
            .ok_or_else(|| WatchlistResolutionError::NotFound {
                query: id_or_name.to_string(),
            }),
        [] => Err(WatchlistResolutionError::NotFound {
            query: id_or_name.to_string(),
        }),
        _ => Err(WatchlistResolutionError::MultipleMatches {
            query: id_or_name.to_string(),
            matches: matches
                .iter()
                .filter_map(|watchlist| {
                    let id = watchlist.id.as_deref()?;
                    let name = watchlist.name.as_deref().unwrap_or("<unnamed>");
                    Some(format!("{name} ({id})"))
                })
                .collect(),
        }),
    }
}

fn render_watchlist_resolution_error(error: WatchlistResolutionError) -> i32 {
    match error {
        WatchlistResolutionError::NotFound { query } => render_no_results_message(
            format!("No watchlist matched '{query}'."),
            Some(
                "Run `marketsurge-agent watchlist list --query <name>` to find the watchlist ID."
                    .to_string(),
            ),
        ),
        WatchlistResolutionError::Empty { query } => render_no_results_message(
            format!("Watchlist '{query}' has no resolved ticker symbols."),
            Some("Add symbols to the watchlist or pass symbols directly.".to_string()),
        ),
        WatchlistResolutionError::MultipleMatches { query, matches } => {
            render_usage_message_with_suggestion(
                format!("Multiple watchlists matched '{query}'."),
                Some(format!(
                    "Use one of these watchlist IDs instead: {}.",
                    matches.join(", ")
                )),
            )
        }
    }
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
                    symbol: watchlist_item_symbol(item),
                    key: item.key.clone(),
                    dow_jones_key: item.dow_jones_key.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn watchlist_item_symbol(item: &crate::watchlist::WatchlistItem) -> Option<String> {
    item.symbol
        .as_deref()
        .filter(|symbol| !symbol.is_empty())
        .map(str::to_string)
        .or_else(|| {
            item.dow_jones_key
                .as_deref()
                .and_then(|key| key.rsplit_once(':').map(|(_, symbol)| symbol))
                .filter(|symbol| !symbol.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            item.key
                .as_deref()
                .filter(|key| looks_like_ticker(key))
                .map(str::to_string)
        })
}

fn watchlist_detail_symbols(watchlist: Option<&WatchlistDetail>) -> Vec<String> {
    flatten_watchlist_symbols(watchlist)
        .into_iter()
        .filter_map(|record| record.symbol)
        .collect()
}

fn looks_like_ticker(value: &str) -> bool {
    (1..=8).contains(&value.len())
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '.' || ch == '-')
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_symbols(args: &WatchlistSymbolsArgs, fields: &[String]) -> i32 {
    let watchlist_id_or_name = args.watchlist_id.clone();

    run_client_command(fields, |client| async move {
        let watchlists = api_call(client.get_all_watchlist_names()).await?;
        let watchlist_id =
            match resolve_watchlist_id_from_response(&watchlists, &watchlist_id_or_name) {
                Ok(watchlist_id) => watchlist_id,
                Err(error) => return Err(render_watchlist_resolution_error(error)),
            };

        let response = api_call(client.flagged_symbols(&watchlist_id)).await?;

        Ok(flatten_watchlist_symbols(response.watchlist.as_ref()))
    })
    .await
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_screen(args: &WatchlistScreenArgs, fields: &[String]) -> i32 {
    let columns = response_columns(&args.columns);
    let symbols = args.symbols.clone();
    let watchlist_id_or_name = args.watchlist_symbols.clone();
    let watchlist_query = watchlist_id_or_name.clone();

    run_client_command(fields, |client| async move {
        let symbols = match watchlist_id_or_name {
            Some(watchlist_id_or_name) => {
                let watchlists = api_call(client.get_all_watchlist_names()).await?;
                let watchlist_id =
                    match resolve_watchlist_id_from_response(&watchlists, &watchlist_id_or_name) {
                        Ok(watchlist_id) => watchlist_id,
                        Err(error) => return Err(render_watchlist_resolution_error(error)),
                    };
                let response = api_call(client.flagged_symbols(&watchlist_id)).await?;
                watchlist_detail_symbols(response.watchlist.as_ref())
            }
            None => symbols,
        };
        if symbols.is_empty() {
            return Err(render_watchlist_resolution_error(
                WatchlistResolutionError::Empty {
                    query: watchlist_query.unwrap_or_else(|| "provided symbols".to_string()),
                },
            ));
        }
        let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
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
    use crate::cli::args::Cli;
    use crate::cli::common::exit::ExitCode;
    use crate::cli::common::test_support::{response_value, response_value_without_md_item};
    use crate::watchlist::WatchlistItem;
    use clap::Parser;

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
                    symbol: Some("AAPL".into()),
                    key: Some("AAPL".into()),
                    dow_jones_key: Some("US:AAPL".into()),
                },
                WatchlistItem {
                    symbol: Some("MSFT".into()),
                    key: Some("MSFT".into()),
                    dow_jones_key: Some("US:MSFT".into()),
                },
            ],
        };

        let records = flatten_watchlist_symbols(Some(&detail));

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].watchlist_id.as_deref(), Some("42"));
        assert_eq!(records[0].watchlist_name.as_deref(), Some("Tech"));
        assert_eq!(records[0].symbol.as_deref(), Some("AAPL"));
        assert_eq!(records[0].key.as_deref(), Some("AAPL"));
        assert_eq!(records[1].symbol.as_deref(), Some("MSFT"));
        assert_eq!(records[1].key.as_deref(), Some("MSFT"));
        assert_eq!(records[1].dow_jones_key.as_deref(), Some("US:MSFT"));
    }

    #[test]
    fn flatten_symbols_derives_symbol_from_dow_jones_key() {
        let detail = WatchlistDetail {
            id: Some("42".into()),
            name: Some("Tech".into()),
            last_modified_date_utc: None,
            description: None,
            items: vec![WatchlistItem {
                symbol: None,
                key: Some("opaque-key".into()),
                dow_jones_key: Some("US:NVDA".into()),
            }],
        };

        let records = flatten_watchlist_symbols(Some(&detail));

        assert_eq!(records[0].symbol.as_deref(), Some("NVDA"));
        assert_eq!(records[0].key.as_deref(), Some("opaque-key"));
    }

    #[test]
    fn flatten_symbols_uses_api_symbol_for_opaque_keys() {
        let detail = WatchlistDetail {
            id: Some("42".into()),
            name: Some("Tech".into()),
            last_modified_date_utc: None,
            description: None,
            items: vec![WatchlistItem {
                symbol: Some("AAPL".into()),
                key: Some("a1b2c3d4".into()),
                dow_jones_key: Some("13-3122".into()),
            }],
        };

        let records = flatten_watchlist_symbols(Some(&detail));

        assert_eq!(records[0].symbol.as_deref(), Some("AAPL"));
        assert_eq!(records[0].key.as_deref(), Some("a1b2c3d4"));
        assert_eq!(records[0].dow_jones_key.as_deref(), Some("13-3122"));
    }

    #[test]
    fn flatten_symbols_does_not_emit_opaque_key_as_symbol() {
        let detail = WatchlistDetail {
            id: Some("42".into()),
            name: Some("Tech".into()),
            last_modified_date_utc: None,
            description: None,
            items: vec![WatchlistItem {
                symbol: None,
                key: Some("a1b2c3d4".into()),
                dow_jones_key: Some("13-3122".into()),
            }],
        };

        let records = flatten_watchlist_symbols(Some(&detail));

        assert_eq!(records[0].symbol, None);
        assert_eq!(records[0].key.as_deref(), Some("a1b2c3d4"));
        assert_eq!(records[0].dow_jones_key.as_deref(), Some("13-3122"));
    }

    #[test]
    fn watchlist_detail_symbols_keeps_only_resolved_tickers() {
        let detail = WatchlistDetail {
            id: Some("42".into()),
            name: Some("Tech".into()),
            last_modified_date_utc: None,
            description: None,
            items: vec![
                WatchlistItem {
                    symbol: Some("AAPL".into()),
                    key: Some("AAPL".into()),
                    dow_jones_key: None,
                },
                WatchlistItem {
                    symbol: None,
                    key: Some("opaque-key".into()),
                    dow_jones_key: Some("US:NVDA".into()),
                },
                WatchlistItem {
                    symbol: None,
                    key: Some("a1b2c3d4".into()),
                    dow_jones_key: Some("13-3122".into()),
                },
            ],
        };

        let symbols = watchlist_detail_symbols(Some(&detail));

        assert_eq!(symbols, vec!["AAPL", "NVDA"]);
    }

    #[test]
    fn watchlist_screen_accepts_watchlist_symbols_flag() {
        let cli = Cli::parse_from([
            "marketsurge-agent",
            "watchlist",
            "screen",
            "--symbols",
            "Growth Picks",
        ]);

        let command = format!("{:?}", cli.command);

        assert!(command.contains("symbols: []"));
        assert!(command.contains("watchlist_symbols: Some(\"Growth Picks\")"));
    }

    #[test]
    fn watchlist_screen_rejects_symbols_and_watchlist_symbols_together() {
        let error = Cli::try_parse_from([
            "marketsurge-agent",
            "watchlist",
            "screen",
            "AAPL",
            "--symbols",
            "Growth Picks",
        ])
        .expect_err("positional symbols and --symbols should conflict");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn resolve_watchlist_id_accepts_exact_id() {
        let response = crate::watchlist::WatchlistNamesResponse {
            watchlists: vec![WatchlistSummary {
                id: Some("12345".into()),
                name: Some("My Watchlist".into()),
                last_modified_date_utc: None,
                description: None,
            }],
        };

        let resolved = resolve_watchlist_id_from_response(&response, "12345");

        assert_eq!(resolved.expect("id should resolve"), "12345");
    }

    #[test]
    fn resolve_watchlist_id_accepts_exact_normalized_name() {
        let response = crate::watchlist::WatchlistNamesResponse {
            watchlists: vec![WatchlistSummary {
                id: Some("12345".into()),
                name: Some("My Watchlist".into()),
                last_modified_date_utc: None,
                description: None,
            }],
        };

        let resolved = resolve_watchlist_id_from_response(&response, "my-watchlist");

        assert_eq!(resolved.expect("name should resolve"), "12345");
    }

    #[test]
    fn resolve_watchlist_id_reports_missing_match() {
        let response = crate::watchlist::WatchlistNamesResponse { watchlists: vec![] };

        let resolved = resolve_watchlist_id_from_response(&response, "missing");

        assert!(matches!(
            resolved,
            Err(WatchlistResolutionError::NotFound { .. })
        ));
    }

    #[test]
    fn resolve_watchlist_id_reports_name_match_without_id_as_missing() {
        let response = crate::watchlist::WatchlistNamesResponse {
            watchlists: vec![WatchlistSummary {
                id: None,
                name: Some("My Watchlist".into()),
                last_modified_date_utc: None,
                description: None,
            }],
        };

        let resolved = resolve_watchlist_id_from_response(&response, "My Watchlist");

        assert!(matches!(
            resolved,
            Err(WatchlistResolutionError::NotFound { query }) if query == "My Watchlist"
        ));
    }

    #[test]
    fn resolve_watchlist_id_reports_multiple_name_matches() {
        let response = crate::watchlist::WatchlistNamesResponse {
            watchlists: vec![
                WatchlistSummary {
                    id: Some("1".into()),
                    name: Some("Jeff Sun".into()),
                    last_modified_date_utc: None,
                    description: None,
                },
                WatchlistSummary {
                    id: Some("2".into()),
                    name: Some("Jeff-Sun".into()),
                    last_modified_date_utc: None,
                    description: None,
                },
            ],
        };

        let resolved = resolve_watchlist_id_from_response(&response, "Jeff Sun");

        assert!(matches!(
            resolved,
            Err(WatchlistResolutionError::MultipleMatches { .. })
        ));
    }

    #[test]
    fn render_watchlist_resolution_error_reports_no_results() {
        let exit_code = render_watchlist_resolution_error(WatchlistResolutionError::NotFound {
            query: "$(rm -rf /)".into(),
        });

        assert_eq!(exit_code, ExitCode::NoResults.code());
    }

    #[test]
    fn render_watchlist_resolution_error_reports_empty_watchlist_as_no_results() {
        let exit_code = render_watchlist_resolution_error(WatchlistResolutionError::Empty {
            query: "Growth Picks".into(),
        });

        assert_eq!(exit_code, ExitCode::NoResults.code());
    }

    #[test]
    fn render_watchlist_resolution_error_reports_multiple_matches_as_usage_error() {
        let exit_code =
            render_watchlist_resolution_error(WatchlistResolutionError::MultipleMatches {
                query: "Jeff Sun".into(),
                matches: vec!["Jeff Sun (1)".into(), "Jeff-Sun (2)".into()],
            });

        assert_eq!(exit_code, ExitCode::Usage.code());
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
        assert_eq!(records[0].get("rs_rating"), Some(&Some("88".into())));
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
