//! Watchlist endpoints for listing, fetching, and screening watchlist items.

use serde::{Deserialize, Serialize};

use crate::adhoc_screen::{AdhocScreenIncludeSource, AdhocScreenInstruments, AdhocScreenResponse};
use crate::client::Client;
use crate::types::{ResponseColumn, symbols_to_owned};

// ---------------------------------------------------------------------------
// GraphQL queries
// ---------------------------------------------------------------------------

const QUERY_GET_ALL_WATCHLIST_NAMES: &str = include_str!("graphql/get_all_watchlist_names.graphql");

const QUERY_FLAGGED_SYMBOLS: &str = include_str!("graphql/flagged_symbols.graphql");

const DEFAULT_WATCHLIST_PUB: &str = "msr";
const DEFAULT_SCREENER_WATCHLIST_CORRELATION_TAG: &str = "Screen With Watchlist";
const DEFAULT_SCREENER_WATCHLIST_DIALECT: &str = "CHARTING";

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GetAllWatchlistNamesVariables {
    #[serde(rename = "pub")]
    publication: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FlaggedSymbolsVariables {
    #[serde(rename = "pub")]
    publication: String,
    watchlist_id: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `GetAllWatchlistNames` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistNamesResponse {
    /// Available watchlists.
    #[serde(default)]
    pub watchlists: Vec<WatchlistSummary>,
}

/// Summary of a single watchlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistSummary {
    /// Watchlist identifier.
    pub id: Option<String>,
    /// Watchlist name.
    pub name: Option<String>,
    /// Last modified timestamp in UTC.
    pub last_modified_date_utc: Option<String>,
    /// Watchlist description.
    pub description: Option<String>,
}

/// Top-level response from the `FlaggedSymbols` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlaggedSymbolsResponse {
    /// Watchlist detail with items.
    pub watchlist: Option<WatchlistDetail>,
}

/// A watchlist with its items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistDetail {
    /// Watchlist identifier.
    pub id: Option<String>,
    /// Watchlist name.
    pub name: Option<String>,
    /// Last modified timestamp in UTC.
    pub last_modified_date_utc: Option<String>,
    /// Watchlist description.
    pub description: Option<String>,
    /// Watchlist symbol items.
    #[serde(default)]
    pub items: Vec<WatchlistItem>,
}

/// A single symbol in a watchlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistItem {
    /// Symbol key (e.g. "AAPL").
    pub key: Option<String>,
    /// Dow Jones symbol key (e.g. "US:AAPL").
    pub dow_jones_key: Option<String>,
}

/// Alias for the screener watchlist items response, which shares the
/// adhoc screen wire format.
pub type ScreenerWatchlistItemsResponse = AdhocScreenResponse;

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches all watchlist names for the default publication.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn get_all_watchlist_names(&self) -> crate::error::Result<WatchlistNamesResponse> {
        let variables = GetAllWatchlistNamesVariables {
            publication: DEFAULT_WATCHLIST_PUB.to_string(),
        };

        self.graphql_operation(
            "GetAllWatchlistNames",
            variables,
            QUERY_GET_ALL_WATCHLIST_NAMES,
        )
        .await
    }

    /// Fetches the symbols in the specified watchlist.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn flagged_symbols(
        &self,
        watchlist_id: &str,
    ) -> crate::error::Result<FlaggedSymbolsResponse> {
        let variables = FlaggedSymbolsVariables {
            publication: DEFAULT_WATCHLIST_PUB.to_string(),
            watchlist_id: watchlist_id.to_string(),
        };

        self.graphql_operation("FlaggedSymbols", variables, QUERY_FLAGGED_SYMBOLS)
            .await
    }

    /// Fetches screener data for specific symbols via the watchlist
    /// screener endpoint.
    ///
    /// This is a convenience wrapper around
    /// [`Client::screener_watchlist`] that fills in default parameters
    /// for the common watchlist-item lookup use case.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn screener_watchlist_items(
        &self,
        symbols: &[&str],
        response_columns: Vec<ResponseColumn>,
    ) -> crate::error::Result<ScreenerWatchlistItemsResponse> {
        let include_source = AdhocScreenIncludeSource {
            screen_id: None,
            instruments: Some(AdhocScreenInstruments {
                symbols: symbols_to_owned(symbols),
                dialect: DEFAULT_SCREENER_WATCHLIST_DIALECT.to_string(),
            }),
        };

        self.screener_watchlist(
            DEFAULT_SCREENER_WATCHLIST_CORRELATION_TAG,
            response_columns,
            include_source,
            1,
            1,
            0,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn get_all_watchlist_names_parses_response() {
        let (_server, client, mock) = mock_test("GetAllWatchlistNames").await;

        let resp = client
            .get_all_watchlist_names()
            .await
            .expect("get_all_watchlist_names should succeed");

        assert_eq!(resp.watchlists.len(), 1);
        let wl = &resp.watchlists[0];
        assert_eq!(wl.id.as_deref(), Some("12345"));
        assert_eq!(wl.name.as_deref(), Some("My Watchlist"));
        assert_eq!(
            wl.last_modified_date_utc.as_deref(),
            Some("2025-01-01T00:00:00Z")
        );
        assert_eq!(wl.description.as_deref(), Some("Test watchlist"));

        mock.assert();
    }

    #[tokio::test]
    async fn flagged_symbols_parses_response() {
        let (_server, client, mock) = mock_test("FlaggedSymbols").await;

        let resp = client
            .flagged_symbols("12345")
            .await
            .expect("flagged_symbols should succeed");

        let watchlist = resp.watchlist.as_ref().expect("watchlist");
        assert_eq!(watchlist.id.as_deref(), Some("12345"));
        assert_eq!(watchlist.name.as_deref(), Some("My Watchlist"));
        assert_eq!(watchlist.items.len(), 2);
        assert_eq!(watchlist.items[0].key.as_deref(), Some("AAPL"));
        assert_eq!(watchlist.items[0].dow_jones_key.as_deref(), Some("US:AAPL"));
        assert_eq!(watchlist.items[1].key.as_deref(), Some("MSFT"));
        assert_eq!(watchlist.items[1].dow_jones_key.as_deref(), Some("US:MSFT"));

        mock.assert();
    }

    #[tokio::test]
    async fn screener_watchlist_items_parses_response() {
        let (_server, client, mock) = mock_test("ScreenerWatchlist").await;

        let columns = vec![
            ResponseColumn {
                name: "EPSRating".to_string(),
                sort_information: None,
            },
            ResponseColumn {
                name: "RSRating".to_string(),
                sort_information: None,
            },
            ResponseColumn {
                name: "AccDisRating".to_string(),
                sort_information: None,
            },
        ];

        let resp = client
            .screener_watchlist_items(&["AMD"], columns)
            .await
            .expect("screener_watchlist_items should succeed");

        let result = resp
            .market_data_adhoc_screen
            .as_ref()
            .expect("market_data_adhoc_screen");
        assert_eq!(
            result.correlation_tag.as_deref(),
            Some("Screen With Watchlist")
        );
        assert_eq!(result.response_values.len(), 1);
        assert_eq!(result.response_values[0].len(), 3);

        let eps = &result.response_values[0][0];
        assert_eq!(eps.value.as_deref(), Some("95"));
        let eps_item = eps.md_item.as_ref().expect("md_item");
        assert_eq!(eps_item.name.as_deref(), Some("EPSRating"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_watchlist() {
        let client = crate::test_support::live_client().await;

        let resp = client
            .get_all_watchlist_names()
            .await
            .expect("live get_all_watchlist_names should succeed");

        assert!(!resp.watchlists.is_empty());
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_flagged_symbols() {
        let client = crate::test_support::live_client().await;

        let watchlists = client
            .get_all_watchlist_names()
            .await
            .expect("live get_all_watchlist_names should succeed");
        let watchlist_id = watchlists
            .watchlists
            .first()
            .and_then(|watchlist| watchlist.id.as_deref())
            .expect("expected at least one live watchlist with an id");

        let resp = client
            .flagged_symbols(watchlist_id)
            .await
            .expect("live flagged_symbols should succeed");

        assert!(resp.watchlist.is_some());
    }
}
