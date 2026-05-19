//! Screen, Screens, RunScreen, and MarketDataScreen endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;
use crate::types::ResponseColumn;

// ---------------------------------------------------------------------------
// GraphQL queries
// ---------------------------------------------------------------------------

const QUERY_SCREEN: &str = r#"query Screen($site: Site!, $screenId: ID!, $coachScreen: Boolean) {
  user {
    screen(site: $site, screenId: $screenId, coachScreen: $coachScreen) {
      id
      name
      site
      description
      filterCriteria
      resultConfig {
        limit
        sortBy {
          field
          direction
        }
      }
      result {
        count
        description
        updatedAt
      }
      type
      source {
        excludeMsrDatabase
      }
      createdAt
      updatedAt
    }
  }
}"#;

const QUERY_SCREENS: &str = r#"query Screens($site: Site!, $type: ScreenType, $sortDir: SortDirInput) {
  user {
    screens(site: $site, type: $type, sortDir: $sortDir) {
      site
      id
      name
      type
      source {
        id
        type
        pub
      }
      updatedAt
      filterCriteria
      description
      createdAt
    }
  }
}"#;

const QUERY_RUN_SCREEN: &str = r#"query RunScreen($input: ScreenResultInput!) {
  user {
    runScreen(input: $input) {
      numberOfMatchingInstruments
      responseValues {
        value
        mdItem {
          name
          mdItemID
        }
      }
    }
  }
}"#;

const QUERY_MARKET_DATA_SCREEN: &str = r#"query MarketDataScreen($screenName: String!, $parameters: [ScreenerParameterInput!]!) {
  marketDataScreen(screenName: $screenName, parameters: $parameters) {
    screenName
    responseValues {
      value
      mdItem {
        mdItemID
        name
      }
    }
    numberOfInstrumentsInSource
    errorValues
    elapsedTime
  }
}"#;

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenVariables {
    site: String,
    screen_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    coach_screen: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreensVariables {
    site: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    screen_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_dir: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunScreenVariables {
    input: RunScreenInput,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MarketDataScreenVariables {
    screen_name: String,
    parameters: Vec<ScreenerParameter>,
}

// ---------------------------------------------------------------------------
// Shared response types (also used by adhoc_screen)
// ---------------------------------------------------------------------------

/// A single cell in a screen response row.
///
/// Contains a string value and metadata identifying the data column.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseValue {
    /// Cell value (e.g. a ticker symbol, numeric string, or rating).
    pub value: Option<String>,
    /// Market data item metadata for this cell.
    pub md_item: Option<MdItem>,
}

/// Identifies a market data column in a screen response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdItem {
    /// Market data item identifier. May be a string or integer on the wire.
    #[serde(rename = "mdItemID")]
    pub md_item_id: Option<serde_json::Value>,
    /// Human-readable column name (e.g. "Symbol", "CompanyName").
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Screen response types
// ---------------------------------------------------------------------------

/// Top-level response from the `Screen` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenResponse {
    /// User-scoped screen data.
    pub user: Option<ScreenUser>,
}

/// User wrapper for a single screen definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenUser {
    /// The requested screen definition.
    pub screen: Option<ScreenDetail>,
}

/// Full definition of a single saved screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenDetail {
    /// Screen identifier.
    pub id: Option<String>,
    /// Screen name.
    pub name: Option<String>,
    /// Site the screen belongs to.
    pub site: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Filter criteria defining the screen's rules.
    pub filter_criteria: Option<ScreenFilterCriteria>,
    /// Result configuration (limit and sort).
    pub result_config: Option<ScreenResultConfig>,
    /// Latest result summary.
    pub result: Option<ScreenResultSummary>,
    /// Screen type (e.g. "STOCK_SCREEN").
    #[serde(rename = "type")]
    pub screen_type: Option<String>,
    /// Source configuration.
    pub source: Option<ScreenDetailSource>,
    /// Creation timestamp.
    pub created_at: Option<String>,
    /// Last update timestamp.
    pub updated_at: Option<String>,
}

/// Filter criteria for a screen definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFilterCriteria {
    /// Filter terms.
    #[serde(default)]
    pub terms: Vec<ScreenFilterTerm>,
    /// Combination type (e.g. "AND").
    #[serde(rename = "type")]
    pub criteria_type: Option<String>,
}

/// A single filter condition within screen criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFilterTerm {
    /// Left-hand side data field.
    pub left: Option<ScreenFilterTermLeft>,
    /// Comparison operator.
    pub operand: Option<String>,
    /// Right-hand side comparison values.
    pub right: Option<ScreenFilterTermRight>,
}

/// Data field on the left side of a filter condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFilterTermLeft {
    /// Field name.
    pub name: Option<String>,
}

/// Comparison values on the right side of a filter condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFilterTermRight {
    /// Exact match value.
    pub value: Option<String>,
    /// Upper bound for range comparisons.
    pub maximum_value: Option<String>,
    /// Lower bound for range comparisons.
    pub minimum_value: Option<String>,
}

/// Result configuration for a screen (limit and sort order).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenResultConfig {
    /// Maximum number of results.
    pub limit: Option<i64>,
    /// Sort specification.
    pub sort_by: Option<ScreenSortBy>,
}

/// Sort field and direction for screen results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSortBy {
    /// Column to sort by.
    pub field: Option<String>,
    /// Sort direction (e.g. "DESCENDING").
    pub direction: Option<String>,
}

/// Summary of the most recent screen run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenResultSummary {
    /// Number of matching instruments.
    pub count: Option<i64>,
    /// Description of the result.
    pub description: Option<String>,
    /// Timestamp of the result.
    pub updated_at: Option<String>,
}

/// Source configuration for a single screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenDetailSource {
    /// Whether the MSR database is excluded.
    pub exclude_msr_database: Option<bool>,
}

// ---------------------------------------------------------------------------
// Screens response types
// ---------------------------------------------------------------------------

/// Top-level response from the `Screens` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreensResponse {
    /// User-scoped screens data.
    pub user: Option<ScreensUser>,
}

/// User wrapper for the screens list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreensUser {
    /// List of saved screen definitions.
    #[serde(default)]
    pub screens: Vec<ScreenEntry>,
}

/// A saved screen definition in a listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenEntry {
    /// Site the screen belongs to.
    pub site: Option<String>,
    /// Screen identifier.
    pub id: Option<String>,
    /// Screen name.
    pub name: Option<String>,
    /// Screen type (e.g. "CUSTOM").
    #[serde(rename = "type")]
    pub screen_type: Option<String>,
    /// Data source linked to this screen.
    pub source: Option<ScreenSource>,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Filter criteria.
    pub filter_criteria: Option<ScreenFilterCriteria>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Creation timestamp.
    pub created_at: Option<String>,
}

/// Data source linked to a saved screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSource {
    /// Source identifier.
    pub id: Option<String>,
    /// Source type.
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    /// Publication identifier.
    #[serde(rename = "pub")]
    pub source_pub: Option<String>,
}

// ---------------------------------------------------------------------------
// RunScreen request types
// ---------------------------------------------------------------------------

/// Data source configuration for a `RunScreen` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenIncludeSource {
    /// Source filter (e.g. "IBD_STOCKS").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Input parameters for a `RunScreen` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenInput {
    /// Correlation tag for request tracking.
    pub correlation_tag: String,
    /// Whether this is a coach account.
    pub coach_account: bool,
    /// Data source filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_source: Option<RunScreenIncludeSource>,
    /// Page size for pagination.
    pub page_size: i64,
    /// Maximum result count.
    pub result_limit: i64,
    /// Screen identifier to run.
    pub screen_id: String,
    /// Site (e.g. "marketsurge").
    pub site: String,
    /// Number of results to skip.
    pub skip: i64,
    /// Columns to include in the response.
    #[serde(default)]
    pub response_columns: Vec<ResponseColumn>,
}

// ---------------------------------------------------------------------------
// RunScreen response types
// ---------------------------------------------------------------------------

/// Top-level response from the `RunScreen` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenResponse {
    /// User-scoped run screen data.
    pub user: Option<RunScreenUser>,
}

/// User wrapper for run screen results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenUser {
    /// Run screen result.
    pub run_screen: Option<RunScreenResult>,
}

/// Result of running a saved screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenResult {
    /// Number of instruments matching the screen criteria.
    pub number_of_matching_instruments: Option<i64>,
    /// 2D array of response values (rows of cells).
    #[serde(default)]
    pub response_values: Vec<Vec<ResponseValue>>,
}

// ---------------------------------------------------------------------------
// MarketDataScreen request types
// ---------------------------------------------------------------------------

/// A key-value parameter for a named screen query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerParameter {
    /// Parameter name (e.g. "DowJonesExchange").
    pub name: String,
    /// Parameter value (e.g. "13").
    pub value: String,
}

// ---------------------------------------------------------------------------
// MarketDataScreen response types
// ---------------------------------------------------------------------------

/// Top-level response from the `MarketDataScreen` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataScreenResponse {
    /// Named screen result data.
    pub market_data_screen: Option<MarketDataScreenResult>,
}

/// Result of a named screen query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataScreenResult {
    /// Screen name echoed from the request.
    pub screen_name: Option<String>,
    /// 2D array of response values (rows of cells).
    #[serde(default)]
    pub response_values: Vec<Vec<ResponseValue>>,
    /// Total instruments in the source universe.
    pub number_of_instruments_in_source: Option<i64>,
    /// Error values from the screener.
    #[serde(default)]
    pub error_values: Vec<String>,
    /// Server-side elapsed time.
    pub elapsed_time: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches a single screen definition by its ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn screen(
        &self,
        site: &str,
        screen_id: &str,
        coach_screen: Option<bool>,
    ) -> crate::error::Result<ScreenResponse> {
        let variables = ScreenVariables {
            site: site.to_string(),
            screen_id: screen_id.to_string(),
            coach_screen,
        };

        let request = GraphQLRequest {
            operation_name: "Screen".to_string(),
            variables,
            query: QUERY_SCREEN.to_string(),
        };

        self.graphql_post(&request).await
    }

    /// Lists saved screen definitions for the authenticated user.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn screens(&self, site: &str) -> crate::error::Result<ScreensResponse> {
        let variables = ScreensVariables {
            site: site.to_string(),
            screen_type: None,
            sort_dir: None,
        };

        let request = GraphQLRequest {
            operation_name: "Screens".to_string(),
            variables,
            query: QUERY_SCREENS.to_string(),
        };

        self.graphql_post(&request).await
    }

    /// Runs a saved screen by its ID and returns matching instruments.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn run_screen(
        &self,
        input: RunScreenInput,
    ) -> crate::error::Result<RunScreenResponse> {
        let variables = RunScreenVariables { input };

        let request = GraphQLRequest {
            operation_name: "RunScreen".to_string(),
            variables,
            query: QUERY_RUN_SCREEN.to_string(),
        };

        self.graphql_post(&request).await
    }

    /// Runs a named screen query with key-value parameters.
    ///
    /// This is used for predefined screens such as
    /// `MarketSurge.RelatedInformation.MUTIFundOwnership` that accept
    /// instrument-identifying parameters rather than freeform filter
    /// criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn market_data_screen(
        &self,
        screen_name: &str,
        parameters: Vec<ScreenerParameter>,
    ) -> crate::error::Result<MarketDataScreenResponse> {
        let variables = MarketDataScreenVariables {
            screen_name: screen_name.to_string(),
            parameters,
        };

        let request = GraphQLRequest {
            operation_name: "MarketDataScreen".to_string(),
            variables,
            query: QUERY_MARKET_DATA_SCREEN.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn screen_parses_response() {
        let (_server, client, mock) = mock_test("Screen").await;

        let resp = client
            .screen("marketsurge", "screen-Peter Lynch", Some(true))
            .await
            .expect("screen should succeed");

        let user = resp.user.as_ref().expect("user");
        let detail = user.screen.as_ref().expect("screen");
        assert_eq!(detail.id.as_deref(), Some("screen-Peter Lynch"));
        assert_eq!(detail.name.as_deref(), Some("Peter Lynch"));
        assert_eq!(detail.screen_type.as_deref(), Some("STOCK_SCREEN"));

        let config = detail.result_config.as_ref().expect("result_config");
        assert_eq!(config.limit, Some(500));
        let sort = config.sort_by.as_ref().expect("sort_by");
        assert_eq!(sort.field.as_deref(), Some("RSRating"));
        assert_eq!(sort.direction.as_deref(), Some("DESCENDING"));

        let result = detail.result.as_ref().expect("result");
        assert_eq!(result.count, Some(42));

        let criteria = detail.filter_criteria.as_ref().expect("filter_criteria");
        assert_eq!(criteria.criteria_type.as_deref(), Some("AND"));
        assert_eq!(criteria.terms.len(), 2);
        assert_eq!(
            criteria.terms[0].left.as_ref().unwrap().name.as_deref(),
            Some("RSRating")
        );

        let source = detail.source.as_ref().expect("source");
        assert_eq!(source.exclude_msr_database, Some(false));

        mock.assert();
    }

    #[tokio::test]
    async fn screens_parses_response() {
        let (_server, client, mock) = mock_test("Screens").await;

        let resp = client
            .screens("marketsurge")
            .await
            .expect("screens should succeed");

        let user = resp.user.as_ref().expect("user");
        assert_eq!(user.screens.len(), 2);

        let first = &user.screens[0];
        assert_eq!(first.id.as_deref(), Some("scr-001"));
        assert_eq!(first.name.as_deref(), Some("Growth Leaders"));
        assert_eq!(first.screen_type.as_deref(), Some("CUSTOM"));

        let source = first.source.as_ref().expect("source");
        assert_eq!(source.id.as_deref(), Some("src-001"));
        assert_eq!(source.source_type.as_deref(), Some("USER"));
        assert_eq!(source.source_pub.as_deref(), Some("msr"));

        let second = &user.screens[1];
        assert_eq!(second.id.as_deref(), Some("scr-002"));
        assert!(second.source.is_none());
        assert!(second.filter_criteria.is_none());

        mock.assert();
    }

    #[tokio::test]
    async fn run_screen_parses_response() {
        let (_server, client, mock) = mock_test("RunScreen").await;

        let input = RunScreenInput {
            correlation_tag: "marketsurge".to_string(),
            coach_account: true,
            include_source: Some(RunScreenIncludeSource { source: None }),
            page_size: 1000,
            result_limit: 1_000_000,
            screen_id: "screen-abc-123".to_string(),
            site: "marketsurge".to_string(),
            skip: 0,
            response_columns: vec![
                ResponseColumn {
                    name: "Symbol".to_string(),
                    sort_information: None,
                },
                ResponseColumn {
                    name: "CompanyName".to_string(),
                    sort_information: None,
                },
            ],
        };

        let resp = client
            .run_screen(input)
            .await
            .expect("run_screen should succeed");

        let user = resp.user.as_ref().expect("user");
        let result = user.run_screen.as_ref().expect("run_screen");
        assert_eq!(result.number_of_matching_instruments, Some(3));

        // Verify 2D array structure
        assert_eq!(result.response_values.len(), 2);
        assert_eq!(result.response_values[0].len(), 2);

        let first_cell = &result.response_values[0][0];
        assert_eq!(first_cell.value.as_deref(), Some("NVDA"));
        let md_item = first_cell.md_item.as_ref().expect("md_item");
        assert_eq!(md_item.name.as_deref(), Some("Symbol"));

        let second_row = &result.response_values[1][0];
        assert_eq!(second_row.value.as_deref(), Some("GOOGL"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_screen() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .screen("marketsurge", "screen-Peter Lynch", Some(true))
            .await
            .expect("live screen should succeed");

        assert!(resp.user.and_then(|user| user.screen).is_some());
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_screens() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .screens("marketsurge")
            .await
            .expect("live screens should succeed");

        let user = resp.user.expect("user");
        assert!(!user.screens.is_empty());
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_run_screen() {
        let client = crate::test_support::live_client().await;
        let input = RunScreenInput {
            correlation_tag: "marketsurge".to_string(),
            coach_account: true,
            include_source: Some(RunScreenIncludeSource { source: None }),
            page_size: 1000,
            result_limit: 1_000_000,
            screen_id: "screen-Peter Lynch".to_string(),
            site: "marketsurge".to_string(),
            skip: 0,
            response_columns: vec![ResponseColumn {
                name: "Symbol".to_string(),
                sort_information: None,
            }],
        };

        let resp = client
            .run_screen(input)
            .await
            .expect("live run_screen should succeed");

        assert!(resp.user.and_then(|user| user.run_screen).is_some());
    }

    #[tokio::test]
    async fn market_data_screen_parses_response() {
        let (_server, client, mock) = mock_test("MarketDataScreen").await;

        let parameters = vec![
            ScreenerParameter {
                name: "DowJonesExchange".to_string(),
                value: "13".to_string(),
            },
            ScreenerParameter {
                name: "DowJonesId".to_string(),
                value: "4698".to_string(),
            },
        ];

        let resp = client
            .market_data_screen(
                "MarketSurge.RelatedInformation.MUTIFundOwnership",
                parameters,
            )
            .await
            .expect("market_data_screen should succeed");

        let result = resp
            .market_data_screen
            .as_ref()
            .expect("market_data_screen");
        assert_eq!(
            result.screen_name.as_deref(),
            Some("MarketSurge.RelatedInformation.MUTIFundOwnership")
        );
        assert_eq!(result.number_of_instruments_in_source, Some(5));
        assert_eq!(result.elapsed_time.as_deref(), Some("PT0.0157223S"));
        assert!(result.error_values.is_empty());

        // Two fund rows in the fixture
        assert_eq!(result.response_values.len(), 2);
        assert_eq!(result.response_values[0].len(), 12);

        // First fund: JPMorgan
        let symbol = &result.response_values[0][0];
        assert_eq!(symbol.value.as_deref(), Some("SEEGX"));
        let md = symbol.md_item.as_ref().expect("md_item");
        assert_eq!(md.name.as_deref(), Some("Symbol"));

        let name = &result.response_values[0][2];
        assert_eq!(
            name.value.as_deref(),
            Some("JPMorgan Large-Cap Growth Fund I Cl")
        );

        let pct = &result.response_values[0][3];
        assert_eq!(pct.value.as_deref(), Some("0.86"));

        // Second fund: T. Rowe Price
        let second_symbol = &result.response_values[1][0];
        assert_eq!(second_symbol.value.as_deref(), Some("PRCOX"));

        // Empty values for JPMorgan's Q3/Q4 ago
        let q3 = &result.response_values[0][8];
        assert_eq!(q3.value.as_deref(), Some(""));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_market_data_screen() {
        let client = crate::test_support::live_client().await;

        let parameters = vec![
            ScreenerParameter {
                name: "DowJonesExchange".to_string(),
                value: "13".to_string(),
            },
            ScreenerParameter {
                name: "DowJonesId".to_string(),
                value: "4698".to_string(),
            },
        ];

        let resp = client
            .market_data_screen(
                "MarketSurge.RelatedInformation.MUTIFundOwnership",
                parameters,
            )
            .await
            .expect("live market_data_screen should succeed");

        assert!(resp.market_data_screen.is_some());
    }
}
