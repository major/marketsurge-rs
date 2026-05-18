//! RS rating and relative strength panel endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;
use crate::types::SymbolVariables;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_RS_RATING_RI_PANEL: &str = r#"query RSRatingRIPanel(
  $symbols: [String!]!
  $symbolDialectType: MDSymbolDialectType!
) {
  marketData(symbols: $symbols, symbolDialectType: $symbolDialectType) {
    id
    originRequest {
      fromDialect
      symbol
    }
    ratings {
      rsRating {
        letterValue
        period
        periodOffset
        value
      }
    }
    pricingStatistics {
      intradayStatistics {
        rsLineNewHigh
      }
    }
  }
}"#;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `RSRatingRIPanel` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingRiPanelResponse {
    /// Per-symbol RS rating data items.
    #[serde(default)]
    pub market_data: Vec<RsRatingRiPanelItem>,
}

/// RS rating data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingRiPanelItem {
    /// Symbol identifier.
    pub id: Option<String>,
    /// Original request metadata.
    pub origin_request: Option<RsRatingOriginRequest>,
    /// RS rating data.
    pub ratings: Option<RsRatingRatings>,
    /// Pricing statistics.
    pub pricing_statistics: Option<RsRatingPricingStatistics>,
}

/// Original request dialect and symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingOriginRequest {
    /// Source dialect (e.g. "CHARTING").
    pub from_dialect: Option<String>,
    /// Ticker symbol.
    pub symbol: Option<String>,
}

/// RS rating data container.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingRatings {
    /// RS rating snapshots across periods.
    #[serde(default)]
    pub rs_rating: Vec<RsRatingSnapshot>,
}

/// A single RS rating value at a specific period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingSnapshot {
    /// Letter grade (e.g. "A", "B").
    pub letter_value: Option<String>,
    /// Period type (e.g. "DAILY").
    pub period: Option<String>,
    /// Period offset (e.g. "CURRENT", "P1W_AGO").
    pub period_offset: Option<String>,
    /// Numeric RS rating value.
    pub value: Option<i64>,
}

/// Pricing statistics container.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingPricingStatistics {
    /// Intraday statistics.
    pub intraday_statistics: Option<RsRatingIntradayStatistics>,
}

/// Intraday statistics data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsRatingIntradayStatistics {
    /// Whether the RS line is at a new high.
    pub rs_line_new_high: Option<bool>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches RS rating panel data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn rs_rating_ri_panel(
        &self,
        symbols: &[&str],
        symbol_dialect_type: Option<&str>,
    ) -> crate::error::Result<RsRatingRiPanelResponse> {
        let request = GraphQLRequest {
            operation_name: "RSRatingRIPanel".to_string(),
            variables: SymbolVariables::new(symbols, symbol_dialect_type),
            query: QUERY_RS_RATING_RI_PANEL.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn rs_rating_ri_panel_parses_response() {
        let (_server, client, mock) = mock_test("RSRatingRIPanel").await;

        let resp = client
            .rs_rating_ri_panel(&["AAPL"], None)
            .await
            .expect("rs_rating_ri_panel should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id.as_deref(), Some("AAPL"));

        let origin = item.origin_request.as_ref().expect("origin_request");
        assert_eq!(origin.from_dialect.as_deref(), Some("CHARTING"));
        assert_eq!(origin.symbol.as_deref(), Some("AAPL"));

        let ratings = item.ratings.as_ref().expect("ratings");
        assert_eq!(ratings.rs_rating.len(), 2);
        assert_eq!(ratings.rs_rating[0].letter_value.as_deref(), Some("A"));
        assert_eq!(ratings.rs_rating[0].period.as_deref(), Some("DAILY"));
        assert_eq!(
            ratings.rs_rating[0].period_offset.as_deref(),
            Some("CURRENT")
        );
        assert_eq!(ratings.rs_rating[0].value, Some(92));

        assert_eq!(ratings.rs_rating[1].letter_value.as_deref(), Some("B"));
        assert_eq!(
            ratings.rs_rating[1].period_offset.as_deref(),
            Some("P1W_AGO")
        );
        assert_eq!(ratings.rs_rating[1].value, Some(85));

        let pricing = item
            .pricing_statistics
            .as_ref()
            .expect("pricing_statistics");
        let intraday = pricing
            .intraday_statistics
            .as_ref()
            .expect("intraday_statistics");
        assert_eq!(intraday.rs_line_new_high, Some(true));

        mock.assert();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_rs_rating_ri_panel() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .rs_rating_ri_panel(&["AAPL"], None)
            .await
            .expect("live rs_rating_ri_panel should succeed");

        assert!(!resp.market_data.is_empty());
    }
}
