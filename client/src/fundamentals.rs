//! Fundamentals financial data endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::types::symbols_to_owned;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_FUNDERMENTAL_DATA_BOX: &str = include_str!("graphql/fundermental_data_box.graphql");

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FundamentalsVariables {
    symbols: Vec<String>,
    symbol_dialect_type: String,
    up_to_historical_period_offset: String,
    up_to_query_period_offset: String,
    #[serde(rename = "reportedSalesUpToHistoricalPeriod2")]
    reported_sales_up_to_historical_period_2: String,
    #[serde(rename = "salesEstimatesUpToQueryPeriod2")]
    sales_estimates_up_to_query_period_2: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `FundermentalDataBox` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsResponse {
    /// Per-symbol fundamental data items.
    #[serde(default)]
    pub market_data: Vec<FundamentalsItem>,
}

/// Fundamental data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsItem {
    /// Symbol identifier.
    pub id: Option<String>,
    /// Financial data (consensus and estimates).
    pub financials: Option<FundamentalsFinancials>,
    /// Company and instrument symbology.
    pub symbology: Option<FundamentalsSymbology>,
}

/// Consensus financials and forward estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsFinancials {
    /// Consensus EPS and sales data.
    pub consensus_financials: Option<FundamentalsConsensus>,
    /// Forward EPS and sales estimates.
    pub estimates: Option<FundamentalsEstimates>,
}

/// Consensus EPS and sales data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsConsensus {
    /// Consensus EPS data.
    pub eps: Option<FundamentalsConsensusEps>,
    /// Consensus sales data.
    pub sales: Option<FundamentalsConsensusSales>,
}

/// Consensus EPS with reported earnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsConsensusEps {
    /// Reported earnings periods.
    #[serde(default)]
    pub reported_earnings: Vec<FundamentalsReportedPeriod>,
}

/// Consensus sales with reported sales.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsConsensusSales {
    /// Reported sales periods.
    #[serde(default)]
    pub reported_sales: Vec<FundamentalsReportedPeriod>,
}

/// A single reported earnings or sales period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsReportedPeriod {
    /// Numeric value with formatted display string.
    pub value: Option<FundamentalsFormattedValue>,
    /// Year-over-year percent change.
    #[serde(rename = "percentChangeYOY")]
    pub percent_change_yoy: Option<FundamentalsFormattedValue>,
    /// Period offset (e.g. "CURRENT", "P1Q_AGO").
    pub period_offset: Option<String>,
    /// Period end date.
    pub period_end_date: Option<FundamentalsDateValue>,
}

/// Forward EPS and sales estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsEstimates {
    /// Forward EPS estimates.
    #[serde(default)]
    pub eps_estimates: Vec<FundamentalsEstimate>,
    /// Forward sales estimates.
    #[serde(default)]
    pub sales_estimates: Vec<FundamentalsEstimate>,
}

/// A single forward earnings or sales estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsEstimate {
    /// Numeric value with formatted display string.
    pub value: Option<FundamentalsFormattedValue>,
    /// Year-over-year percent change.
    #[serde(rename = "percentChangeYOY")]
    pub percent_change_yoy: Option<FundamentalsFormattedValue>,
    /// Period offset (e.g. "P1Q_FUTURE").
    pub period_offset: Option<String>,
    /// Period identifier (e.g. "P1Q").
    pub period: Option<String>,
    /// Estimate revision direction (e.g. "UP", "DOWN").
    pub revision_direction: Option<String>,
}

/// Numeric value with formatted display string.
pub type FundamentalsFormattedValue = crate::types::FormattedFloat;

/// Single date value.
pub type FundamentalsDateValue = crate::types::DateValue;

/// Company and instrument symbology.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsSymbology {
    /// Company information.
    pub company: Option<FundamentalsCompany>,
    /// Instrument information.
    pub instrument: Option<FundamentalsInstrument>,
}

/// Company profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsCompany {
    /// Company name.
    pub company_name: Option<String>,
}

/// Instrument metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsInstrument {
    /// Available symbols.
    #[serde(default)]
    pub symbols: Vec<FundamentalsSymbol>,
}

/// Symbol value and dialect type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsSymbol {
    /// Symbol ticker value.
    pub value: Option<String>,
    /// Symbol dialect type (e.g. "CHARTING").
    #[serde(rename = "type")]
    pub node_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches fundamental financial data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn fundamentals(
        &self,
        symbols: &[&str],
        symbol_dialect_type: &str,
        historical_period_offset: &str,
        query_period_offset: &str,
        reported_sales_period: &str,
        sales_estimates_period: &str,
    ) -> crate::error::Result<FundamentalsResponse> {
        let variables = FundamentalsVariables {
            symbols: symbols_to_owned(symbols),
            symbol_dialect_type: symbol_dialect_type.to_string(),
            up_to_historical_period_offset: historical_period_offset.to_string(),
            up_to_query_period_offset: query_period_offset.to_string(),
            reported_sales_up_to_historical_period_2: reported_sales_period.to_string(),
            sales_estimates_up_to_query_period_2: sales_estimates_period.to_string(),
        };

        self.graphql_operation(
            "FundermentalDataBox",
            variables,
            QUERY_FUNDERMENTAL_DATA_BOX,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn fundamentals_parses_response() {
        let (_server, client, mock) = mock_test("FundermentalDataBox").await;

        let resp = client
            .fundamentals(
                &["AAPL"],
                "CHARTING",
                "P7Y_AGO",
                "P2Y_FUTURE",
                "P7Y_AGO",
                "P2Y_FUTURE",
            )
            .await
            .expect("fundamentals should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id.as_deref(), Some("AAPL"));

        let financials = item.financials.as_ref().expect("financials");
        let consensus = financials
            .consensus_financials
            .as_ref()
            .expect("consensus_financials");

        // Consensus EPS
        let eps = consensus.eps.as_ref().expect("eps");
        assert_eq!(eps.reported_earnings.len(), 2);
        assert_eq!(
            eps.reported_earnings[0].value.as_ref().unwrap().value,
            Some(1.65)
        );
        assert_eq!(
            eps.reported_earnings[0].period_offset.as_deref(),
            Some("CURRENT")
        );
        assert_eq!(
            eps.reported_earnings[0]
                .period_end_date
                .as_ref()
                .unwrap()
                .value
                .as_deref(),
            Some("2026-03-31")
        );

        // Consensus sales
        let sales = consensus.sales.as_ref().expect("sales");
        assert_eq!(sales.reported_sales.len(), 1);
        assert_eq!(
            sales.reported_sales[0]
                .value
                .as_ref()
                .unwrap()
                .formatted_value
                .as_deref(),
            Some("$95.2B")
        );

        // EPS estimates
        let estimates = financials.estimates.as_ref().expect("estimates");
        assert_eq!(estimates.eps_estimates.len(), 1);
        assert_eq!(
            estimates.eps_estimates[0].revision_direction.as_deref(),
            Some("UP")
        );
        assert_eq!(
            estimates.eps_estimates[0].value.as_ref().unwrap().value,
            Some(1.72)
        );

        // Sales estimates
        assert_eq!(estimates.sales_estimates.len(), 1);
        assert_eq!(estimates.sales_estimates[0].period.as_deref(), Some("P1Q"));

        // Symbology
        let symbology = item.symbology.as_ref().expect("symbology");
        let company = symbology.company.as_ref().expect("company");
        assert_eq!(company.company_name.as_deref(), Some("Apple Inc."));

        let instrument = symbology.instrument.as_ref().expect("instrument");
        assert_eq!(instrument.symbols.len(), 1);
        assert_eq!(instrument.symbols[0].value.as_deref(), Some("AAPL"));
        assert_eq!(instrument.symbols[0].node_type.as_deref(), Some("CHARTING"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_fundamentals() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .fundamentals(
                &["AAPL"],
                "CHARTING",
                "P7Y_AGO",
                "P2Y_FUTURE",
                "P7Y_AGO",
                "P2Y_FUTURE",
            )
            .await
            .expect("live fundamentals should succeed");

        assert!(!resp.market_data.is_empty());
    }
}
