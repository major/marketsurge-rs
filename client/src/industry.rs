//! Industry group relative strength and industry overview endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::market_data::{MdFormattedFloat, MdGroupRank, MdPercentChangeVs};
use crate::types::SymbolVariables;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_INDUSTRY_GROUP_RS: &str = include_str!("graphql/industry_group_rs.graphql");

const QUERY_INDUSTRY_OVERVIEW: &str = include_str!("graphql/industry_overview.graphql");

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `IndustryGroupRS` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryGroupRsResponse {
    /// Per-symbol industry group RS data items.
    #[serde(default)]
    pub market_data: Vec<IndustryGroupRsItem>,
}

/// Industry group RS data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryGroupRsItem {
    /// Original request metadata.
    pub origin_request: Option<IndustryGroupRsOriginRequest>,
    /// Industry data.
    pub industry: Option<IndustryGroupRsIndustry>,
}

/// Original request symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryGroupRsOriginRequest {
    /// Ticker symbol.
    pub symbol: Option<String>,
}

/// Industry data including group RS values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryGroupRsIndustry {
    /// Group relative strength ratings.
    #[serde(default, rename = "groupRS")]
    pub group_rs: Vec<IndustryGroupRsValue>,
}

/// A single group RS rating value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryGroupRsValue {
    /// Numeric RS value.
    pub value: Option<i64>,
}

/// Top-level response from the `IndustryOverview` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryOverviewResponse {
    /// Per-symbol industry overview data.
    #[serde(default)]
    pub market_data: Vec<IndustryOverviewItem>,
}

/// Industry overview data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryOverviewItem {
    /// Symbol identifier.
    pub id: Option<String>,
    /// Industry data.
    pub industry: Option<IndustryOverviewIndustry>,
    /// Ratings data including stock rank within industry.
    pub ratings: Option<IndustryOverviewRatings>,
}

/// Industry details including summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryOverviewIndustry {
    /// Industry group name.
    pub name: Option<String>,
    /// Numeric industry code.
    pub ind_code: Option<i64>,
    /// News code identifier.
    pub news_code: Option<String>,
    /// Sector name.
    pub sector: Option<String>,
    /// Group market value in billions.
    pub group_market_value_in_billions: Option<MdFormattedFloat>,
    /// Number of stocks at new highs in the group.
    pub num_new_highs_in_group: Option<i64>,
    /// Number of stocks at new lows in the group.
    pub num_new_lows_in_group: Option<i64>,
    /// Total number of stocks in the group.
    pub number_of_stocks_in_group: Option<i64>,
    /// Industry group rank values.
    #[serde(default)]
    pub group_ranks: Vec<MdGroupRank>,
    /// Price percent change vs various periods.
    #[serde(default)]
    pub price_percent_change_vs: Vec<MdPercentChangeVs>,
}

/// Ratings data with industry rank information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryOverviewRatings {
    /// Whether ratings data is available.
    pub has_ratings_data: Option<bool>,
    /// Stock rank within industry group.
    pub industry: Option<IndustryRankInGroup>,
}

/// Stock's rank within its industry group across rating categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryRankInGroup {
    /// Accumulation/Distribution rank within industry group.
    pub ad_rank_in_industry_group: Option<i64>,
    /// Composite rank within industry group.
    pub comp_rank_in_industry_group: Option<i64>,
    /// EPS rank within industry group.
    pub eps_rank_in_industry_group: Option<i64>,
    /// Total number of stocks in the group.
    pub number_of_stocks_in_group: Option<i64>,
    /// RS rank within industry group.
    pub rs_rank_in_industry_group: Option<i64>,
    /// SMR rank within industry group.
    pub smr_rank_in_industry_group: Option<i64>,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches industry group relative strength data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn industry_group_rs(
        &self,
        symbols: &[&str],
        symbol_dialect_type: Option<&str>,
    ) -> crate::error::Result<IndustryGroupRsResponse> {
        self.graphql_operation(
            "IndustryGroupRS",
            SymbolVariables::new(symbols, symbol_dialect_type),
            QUERY_INDUSTRY_GROUP_RS,
        )
        .await
    }

    /// Fetches industry overview data including summary statistics and
    /// stock rank within industry group.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn industry_overview(
        &self,
        symbols: &[&str],
        symbol_dialect_type: Option<&str>,
    ) -> crate::error::Result<IndustryOverviewResponse> {
        self.graphql_operation(
            "IndustryOverview",
            SymbolVariables::new(symbols, symbol_dialect_type),
            QUERY_INDUSTRY_OVERVIEW,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{mock_test, mock_test_with_fixture};

    #[tokio::test]
    async fn industry_group_rs_parses_response() {
        let (_server, client, mock) = mock_test("IndustryGroupRS").await;

        let resp = client
            .industry_group_rs(&["AAPL"], None)
            .await
            .expect("industry_group_rs should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];

        let origin = item.origin_request.as_ref().expect("origin_request");
        assert_eq!(origin.symbol.as_deref(), Some("AAPL"));

        let industry = item.industry.as_ref().expect("industry");
        assert_eq!(industry.group_rs.len(), 1);
        assert_eq!(industry.group_rs[0].value, Some(85));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_industry_group_rs() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .industry_group_rs(&["AAPL"], None)
            .await
            .expect("live industry_group_rs should succeed");

        assert!(!resp.market_data.is_empty());
    }

    #[tokio::test]
    async fn industry_overview_parses_response() {
        let (_server, client, mock) = mock_test("IndustryOverview").await;

        let resp = client
            .industry_overview(&["13-4698"], Some("DJ_KEY"))
            .await
            .expect("industry_overview should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id.as_deref(), Some("13-4698"));

        // Industry summary
        let industry = item.industry.as_ref().expect("industry");
        assert_eq!(industry.name.as_deref(), Some("Elec-Semicondctor Fablss"));
        assert_eq!(industry.ind_code, Some(7010));
        assert_eq!(industry.news_code.as_deref(), Some("I/SEMF"));
        assert_eq!(industry.sector.as_deref(), Some("CHIPS"));
        assert_eq!(
            industry
                .group_market_value_in_billions
                .as_ref()
                .and_then(|v| v.formatted_value.as_deref()),
            Some("9,056.41B")
        );
        assert_eq!(industry.num_new_highs_in_group, Some(1));
        assert_eq!(industry.num_new_lows_in_group, Some(0));
        assert_eq!(industry.number_of_stocks_in_group, Some(45));

        assert_eq!(industry.group_ranks.len(), 1);
        assert_eq!(industry.group_ranks[0].value, Some(16));
        assert_eq!(
            industry.group_ranks[0].period_offset.as_deref(),
            Some("CURRENT")
        );

        assert_eq!(industry.price_percent_change_vs.len(), 2);
        let today = industry
            .price_percent_change_vs
            .iter()
            .find(|v| v.subject.as_deref() == Some("VS_1D_AGO"))
            .expect("VS_1D_AGO entry");
        assert_eq!(today.formatted_value.as_deref(), Some("-1.22%"));
        let ytd = industry
            .price_percent_change_vs
            .iter()
            .find(|v| v.subject.as_deref() == Some("VS_YTD"))
            .expect("VS_YTD entry");
        assert_eq!(ytd.formatted_value.as_deref(), Some("27.07%"));

        // Stock rank in industry group
        let ratings = item.ratings.as_ref().expect("ratings");
        assert_eq!(ratings.has_ratings_data, Some(true));
        let rank = ratings.industry.as_ref().expect("industry rank");
        assert_eq!(rank.eps_rank_in_industry_group, Some(4));
        assert_eq!(rank.rs_rank_in_industry_group, Some(2));
        assert_eq!(rank.ad_rank_in_industry_group, Some(10));
        assert_eq!(rank.smr_rank_in_industry_group, Some(8));
        assert_eq!(rank.comp_rank_in_industry_group, Some(1));
        assert_eq!(rank.number_of_stocks_in_group, Some(45));

        mock.assert();
    }

    #[tokio::test]
    async fn top_industries_in_sector_parses_response() {
        let (_server, client, mock) =
            mock_test_with_fixture("TopIndustriesInSector", "MarketDataScreen").await;

        let params = vec![crate::screen::ScreenerParameter {
            name: "SectorName".to_string(),
            value: "CHIPS".to_string(),
        }];

        let resp = client
            .market_data_screen(
                "MarketSurge.RelatedInformation.StockTopIndustriesInSector",
                params,
            )
            .await
            .expect("market_data_screen should succeed");

        let result = resp
            .market_data_screen
            .as_ref()
            .expect("market_data_screen result");
        assert_eq!(
            result.screen_name.as_deref(),
            Some("MarketSurge.RelatedInformation.StockTopIndustriesInSector")
        );
        assert_eq!(result.response_values.len(), 3);

        // First row: Elec-Semicondctor Fablss with A+
        let row = &result.response_values[0];
        assert_eq!(row.len(), 2);
        assert_eq!(row[0].value.as_deref(), Some("Elec-Semicondctor Fablss"));
        assert_eq!(row[1].value.as_deref(), Some("A+"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_industry_overview() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .industry_overview(&["AAPL"], None)
            .await
            .expect("live industry_overview should succeed");

        assert!(!resp.market_data.is_empty());
    }
}
