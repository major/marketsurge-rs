//! Ad-hoc screen and screener watchlist endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;
pub use crate::screen::{MdItem, ResponseValue};
use crate::types::ResponseColumn;

// ---------------------------------------------------------------------------
// GraphQL queries
// ---------------------------------------------------------------------------

const QUERY_MARKET_DATA_ADHOC_SCREEN: &str = r#"query MarketDataAdhocScreen(
  $correlationTag: String!
  $adhocQuery: MDAdhocQueryInput
  $responseColumns: [MDAdhocScreenerDataItemInput!]!
  $resultLimit: Int!
  $pageSize: Int!
  $pageSkip: Int
  $includeSource: MDScreenerDataSourceInput!
  $resultType: MDScreenerResultType
) {
  marketDataAdhocScreen(
    correlationTag: $correlationTag
    adhocQuery: $adhocQuery
    resultLimit: $resultLimit
    pageSize: $pageSize
    pageSkip: $pageSkip
    includeSource: $includeSource
    responseDataPoints: $responseColumns
    resultType: $resultType
  ) {
    correlationTag
    elapsedTime
    errorValues
    numberOfInstrumentsInSource
    numberOfMatchingInstruments
    adhocQueryString
    adhocQuery {
      terms {
        numberOfMatchingInstruments
        ordinal
        left {
          name
          mdItemID
        }
        operand
        right {
          value
          maximumValue
          minimumValue
        }
      }
    }
    responseValues {
      value
      mdItem {
        mdItemID
        name
      }
    }
  }
}"#;

const QUERY_SCREENER_WATCHLIST: &str = r#"query ScreenerWatchlist($correlationTag: String!, $responseColumns: [MDAdhocScreenerDataItemInput!]!, $resultLimit: Int!, $pageSize: Int!, $pageSkip: Int, $includeSource: MDScreenerDataSourceInput!) {
  marketDataAdhocScreen(
    correlationTag: $correlationTag
    resultLimit: $resultLimit
    pageSize: $pageSize
    pageSkip: $pageSkip
    includeSource: $includeSource
    responseDataPoints: $responseColumns
  ) {
    correlationTag
    elapsedTime
    errorValues
    numberOfMatchingInstruments
    numberOfInstrumentsInSource
    responseValues {
      value
      mdItem {
        mdItemID
        name
        __typename
      }
      __typename
    }
    __typename
  }
}"#;

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdhocScreenVariables {
    correlation_tag: String,
    response_columns: Vec<ResponseColumn>,
    adhoc_query: Option<serde_json::Value>,
    include_source: AdhocScreenIncludeSource,
    page_size: i64,
    result_limit: i64,
    page_skip: i64,
    result_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenerWatchlistVariables {
    correlation_tag: String,
    response_columns: Vec<ResponseColumn>,
    result_limit: i64,
    page_size: i64,
    page_skip: i64,
    include_source: AdhocScreenIncludeSource,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Data source for an adhoc screen query.
///
/// Use `screen_id` to run a predefined report, or `instruments` to
/// screen specific symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocScreenIncludeSource {
    /// Predefined screen by ID and dialect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_id: Option<AdhocScreenId>,
    /// Instrument-based source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruments: Option<AdhocScreenInstruments>,
}

/// Identifies a predefined screen by ID and dialect.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocScreenId {
    /// Numeric screen identifier.
    pub id: i64,
    /// Dialect (e.g. "MS_LIST_ID").
    pub dialect: String,
}

/// Instrument-based source for an adhoc screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocScreenInstruments {
    /// Ticker symbols.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Symbol dialect (e.g. "CHARTING").
    pub dialect: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `MarketDataAdhocScreen` and
/// `ScreenerWatchlist` queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocScreenResponse {
    /// Adhoc screen result data.
    pub market_data_adhoc_screen: Option<AdhocScreenResult>,
}

/// Result of an adhoc screen query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocScreenResult {
    /// Correlation tag echoed from the request.
    pub correlation_tag: Option<String>,
    /// Server-side elapsed time.
    pub elapsed_time: Option<String>,
    /// Error values from the screener.
    #[serde(default)]
    pub error_values: Vec<String>,
    /// Total instruments in the source universe.
    pub number_of_instruments_in_source: Option<i64>,
    /// Number of instruments matching the query.
    pub number_of_matching_instruments: Option<i64>,
    /// String representation of the adhoc query.
    pub adhoc_query_string: Option<String>,
    /// Parsed adhoc query terms.
    pub adhoc_query: Option<AdhocQueryResult>,
    /// 2D array of response values (rows of cells).
    #[serde(default)]
    pub response_values: Vec<Vec<ResponseValue>>,
}

/// Parsed filter criteria from an adhoc screen query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocQueryResult {
    /// Filter terms.
    #[serde(default)]
    pub terms: Vec<AdhocQueryTerm>,
}

/// A single filter term in an adhoc screen query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocQueryTerm {
    /// Number of instruments matching this term.
    pub number_of_matching_instruments: Option<i64>,
    /// Term ordinal position.
    pub ordinal: Option<i64>,
    /// Left-hand side data field.
    pub left: Option<AdhocQueryTermLeft>,
    /// Comparison operator.
    pub operand: Option<String>,
    /// Right-hand side comparison values.
    pub right: Option<AdhocQueryTermRight>,
}

/// Left-hand side of an adhoc query filter term.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocQueryTermLeft {
    /// Data item name.
    pub name: Option<String>,
    /// Market data item identifier.
    #[serde(rename = "mdItemID")]
    pub md_item_id: Option<serde_json::Value>,
}

/// Right-hand side of an adhoc query filter term.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdhocQueryTermRight {
    /// Exact match value.
    pub value: Option<String>,
    /// Upper bound for range comparisons.
    pub maximum_value: Option<String>,
    /// Lower bound for range comparisons.
    pub minimum_value: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Runs an ad-hoc screen query against the MarketSurge screener.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    #[allow(clippy::too_many_arguments)]
    pub async fn market_data_adhoc_screen(
        &self,
        correlation_tag: &str,
        response_columns: Vec<ResponseColumn>,
        adhoc_query: Option<serde_json::Value>,
        include_source: AdhocScreenIncludeSource,
        page_size: i64,
        result_limit: i64,
        page_skip: i64,
        result_type: &str,
    ) -> crate::error::Result<AdhocScreenResponse> {
        let variables = AdhocScreenVariables {
            correlation_tag: correlation_tag.to_string(),
            response_columns,
            adhoc_query,
            include_source,
            page_size,
            result_limit,
            page_skip,
            result_type: result_type.to_string(),
        };

        let request = GraphQLRequest {
            operation_name: "MarketDataAdhocScreen".to_string(),
            variables,
            query: QUERY_MARKET_DATA_ADHOC_SCREEN.to_string(),
        };

        self.graphql_post(&request).await
    }

    /// Fetches screener values for specific instruments via a watchlist query.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn screener_watchlist(
        &self,
        correlation_tag: &str,
        response_columns: Vec<ResponseColumn>,
        include_source: AdhocScreenIncludeSource,
        result_limit: i64,
        page_size: i64,
        page_skip: i64,
    ) -> crate::error::Result<AdhocScreenResponse> {
        let variables = ScreenerWatchlistVariables {
            correlation_tag: correlation_tag.to_string(),
            response_columns,
            result_limit,
            page_size,
            page_skip,
            include_source,
        };

        let request = GraphQLRequest {
            operation_name: "ScreenerWatchlist".to_string(),
            variables,
            query: QUERY_SCREENER_WATCHLIST.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn market_data_adhoc_screen_parses_response() {
        let (_server, client, mock) = mock_test("MarketDataAdhocScreen").await;

        let columns = vec![
            ResponseColumn {
                name: "Symbol".to_string(),
                sort_information: None,
            },
            ResponseColumn {
                name: "CompanyName".to_string(),
                sort_information: None,
            },
        ];

        let include_source = AdhocScreenIncludeSource {
            screen_id: Some(AdhocScreenId {
                id: 46,
                dialect: "MS_LIST_ID".to_string(),
            }),
            instruments: None,
        };

        let resp = client
            .market_data_adhoc_screen(
                "marketsurge",
                columns,
                None,
                include_source,
                1000,
                1_000_000,
                0,
                "RESULT_WITH_EXPRESSION_COUNTS",
            )
            .await
            .expect("market_data_adhoc_screen should succeed");

        let result = resp
            .market_data_adhoc_screen
            .as_ref()
            .expect("market_data_adhoc_screen");
        assert_eq!(result.correlation_tag.as_deref(), Some("marketsurge"));
        assert_eq!(result.elapsed_time.as_deref(), Some("42ms"));
        assert_eq!(result.number_of_instruments_in_source, Some(8500));
        assert_eq!(result.number_of_matching_instruments, Some(2));
        assert_eq!(
            result.adhoc_query_string.as_deref(),
            Some("CompositeRating >= 90")
        );

        // Verify adhoc query terms
        let query = result.adhoc_query.as_ref().expect("adhoc_query");
        assert_eq!(query.terms.len(), 1);
        assert_eq!(query.terms[0].ordinal, Some(1));
        assert_eq!(
            query.terms[0].left.as_ref().unwrap().name.as_deref(),
            Some("CompositeRating")
        );
        assert_eq!(query.terms[0].operand.as_deref(), Some(">="));

        // Verify 2D response values
        assert_eq!(result.response_values.len(), 2);
        assert_eq!(result.response_values[0].len(), 2);

        let first_cell = &result.response_values[0][0];
        assert_eq!(first_cell.value.as_deref(), Some("AAPL"));
        let md_item = first_cell.md_item.as_ref().expect("md_item");
        assert_eq!(md_item.name.as_deref(), Some("Symbol"));

        let second_row = &result.response_values[1][0];
        assert_eq!(second_row.value.as_deref(), Some("MSFT"));

        mock.assert();
    }

    #[tokio::test]
    async fn screener_watchlist_parses_response() {
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

        let include_source = AdhocScreenIncludeSource {
            screen_id: None,
            instruments: Some(AdhocScreenInstruments {
                symbols: vec!["AMD".to_string()],
                dialect: "CHARTING".to_string(),
            }),
        };

        let resp = client
            .screener_watchlist("Screen With Watchlist", columns, include_source, 1, 1, 0)
            .await
            .expect("screener_watchlist should succeed");

        let result = resp
            .market_data_adhoc_screen
            .as_ref()
            .expect("market_data_adhoc_screen");
        assert_eq!(
            result.correlation_tag.as_deref(),
            Some("Screen With Watchlist")
        );
        assert_eq!(result.number_of_matching_instruments, Some(0));
        assert_eq!(result.number_of_instruments_in_source, Some(0));

        // Verify 2D response values
        assert_eq!(result.response_values.len(), 1);
        assert_eq!(result.response_values[0].len(), 3);

        let eps = &result.response_values[0][0];
        assert_eq!(eps.value.as_deref(), Some("95"));
        let eps_item = eps.md_item.as_ref().expect("md_item");
        assert_eq!(eps_item.name.as_deref(), Some("EPSRating"));

        let rs = &result.response_values[0][1];
        assert_eq!(rs.value.as_deref(), Some("98"));

        let acc = &result.response_values[0][2];
        assert_eq!(acc.value.as_deref(), Some("A+"));

        mock.assert();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_adhoc_screen() {
        let client = crate::test_support::live_client().await;

        let columns = vec![ResponseColumn {
            name: "Symbol".to_string(),
            sort_information: None,
        }];

        let include_source = AdhocScreenIncludeSource {
            screen_id: None,
            instruments: Some(AdhocScreenInstruments {
                symbols: vec!["AAPL".to_string()],
                dialect: "CHARTING".to_string(),
            }),
        };

        let resp = client
            .screener_watchlist("test", columns, include_source, 1, 1, 0)
            .await
            .expect("live screener_watchlist should succeed");

        assert!(resp.market_data_adhoc_screen.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn integration_market_data_adhoc_screen() {
        let client = crate::test_support::live_client().await;

        let columns = vec![ResponseColumn {
            name: "Symbol".to_string(),
            sort_information: None,
        }];

        let include_source = AdhocScreenIncludeSource {
            screen_id: Some(AdhocScreenId {
                id: 46,
                dialect: "MS_LIST_ID".to_string(),
            }),
            instruments: None,
        };

        let resp = client
            .market_data_adhoc_screen(
                "marketsurge",
                columns,
                None,
                include_source,
                1000,
                1_000_000,
                0,
                "RESULT_WITH_EXPRESSION_COUNTS",
            )
            .await
            .expect("live market_data_adhoc_screen should succeed");

        assert!(resp.market_data_adhoc_screen.is_some());
    }
}
