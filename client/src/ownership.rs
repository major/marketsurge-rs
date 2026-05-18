//! Fund ownership data endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;
use crate::types::SymbolVariables;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_OWNERSHIP: &str = r#"query Ownership($symbols: [String!]!, $symbolDialectType: MDSymbolDialectType!) {
  marketData(symbols: $symbols, symbolDialectType: $symbolDialectType) {
    ownership {
      fundsFloatPercentHeld {
        formattedValue
      }
      fundOwnershipSummary {
        date {
          value
        }
        numberOfFundsHeld {
          formattedValue
        }
      }
    }
  }
}"#;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `Ownership` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipResponse {
    /// Per-symbol ownership data.
    #[serde(default)]
    pub market_data: Vec<OwnershipItem>,
}

/// Ownership data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipItem {
    /// Fund ownership statistics.
    pub ownership: Option<OwnershipData>,
}

/// Fund ownership statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipData {
    /// Percentage of float held by funds.
    pub funds_float_percent_held: Option<OwnershipFormattedValue>,
    /// Quarterly fund ownership summaries.
    #[serde(default)]
    pub fund_ownership_summary: Vec<OwnershipQuarterlySummary>,
}

/// A formatted string value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipFormattedValue {
    /// Display-formatted value.
    pub formatted_value: Option<String>,
}

/// Fund ownership data for a single quarter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipQuarterlySummary {
    /// Quarter date.
    pub date: Option<OwnershipDateValue>,
    /// Number of funds holding the stock.
    pub number_of_funds_held: Option<OwnershipFormattedValue>,
}

/// A single date value.
pub type OwnershipDateValue = crate::types::DateValue;

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches fund ownership data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn ownership(&self, symbols: &[&str]) -> crate::error::Result<OwnershipResponse> {
        let request = GraphQLRequest {
            operation_name: "Ownership".to_string(),
            variables: SymbolVariables::new(symbols, None),
            query: QUERY_OWNERSHIP.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn ownership_parses_response() {
        let (_server, client, mock) = mock_test("Ownership").await;

        let resp = client
            .ownership(&["AAPL"])
            .await
            .expect("ownership should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        let ownership = item.ownership.as_ref().expect("ownership");

        // Funds float percent held
        let pct = ownership
            .funds_float_percent_held
            .as_ref()
            .expect("funds_float_percent_held");
        assert_eq!(pct.formatted_value.as_deref(), Some("62.3%"));

        // Fund ownership summary
        assert_eq!(ownership.fund_ownership_summary.len(), 2);
        let q1 = &ownership.fund_ownership_summary[0];
        assert_eq!(
            q1.date.as_ref().unwrap().value.as_deref(),
            Some("2026-03-31")
        );
        assert_eq!(
            q1.number_of_funds_held
                .as_ref()
                .unwrap()
                .formatted_value
                .as_deref(),
            Some("5,432")
        );

        let q2 = &ownership.fund_ownership_summary[1];
        assert_eq!(
            q2.date.as_ref().unwrap().value.as_deref(),
            Some("2025-12-31")
        );
        assert_eq!(
            q2.number_of_funds_held
                .as_ref()
                .unwrap()
                .formatted_value
                .as_deref(),
            Some("5,210")
        );

        mock.assert();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_ownership() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .ownership(&["AAPL"])
            .await
            .expect("live ownership should succeed");

        assert!(!resp.market_data.is_empty());
    }
}
