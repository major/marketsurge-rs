//! Chart market data endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::types::{
    DEFAULT_STRING_KEYS, deserialize_optional_string, json_value_to_string, symbols_to_owned,
};

// ---------------------------------------------------------------------------
// GraphQL queries
// ---------------------------------------------------------------------------

const QUERY_CHART_MARKET_DATA: &str = include_str!("graphql/chart_market_data.graphql");

const QUERY_CHART_MARKET_DATA_WEEKLY: &str =
    include_str!("graphql/chart_market_data_weekly.graphql");

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChartMarketDataVariables {
    symbols: Vec<String>,
    symbol_dialect_type: String,
    #[serde(rename = "where")]
    filter: TimeSeriesFilterInput,
    exchange_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChartMarketDataWeeklyVariables {
    symbols: Vec<String>,
    symbol_dialect_type: String,
    #[serde(rename = "where")]
    filter: TimeSeriesFilterInput,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeSeriesFilterInput {
    start_date_time: EqFilter,
    end_date_time: EqFilter,
    time_series_type: EqFilter,
    include_intraday_data: bool,
}

#[derive(Serialize)]
struct EqFilter {
    eq: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `ChartMarketData` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartMarketDataResponse {
    /// Per-symbol chart market data items.
    #[serde(default)]
    pub market_data: Vec<ChartMarketDataItem>,
    /// Exchange information (present for daily, absent for weekly).
    pub exchange_data: Option<ChartExchangeData>,
}

/// Chart market data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartMarketDataItem {
    /// Symbol identifier.
    pub id: String,
    /// Original request dialect and symbol.
    pub origin_request: Option<ChartOriginRequest>,
    /// Pricing data.
    pub pricing: Option<ChartPricing>,
}

/// Original request dialect and symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartOriginRequest {
    /// Source dialect (e.g. "CHARTING").
    pub from_dialect: String,
    /// Ticker symbol.
    pub symbol: String,
}

/// Pricing data including time series and quotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPricing {
    /// Historical time series data.
    pub time_series: Option<ChartTimeSeries>,
    /// Regular-hours quote.
    pub quote: Option<ChartQuote>,
    /// Pre-market quote.
    pub premarket_quote: Option<ChartQuote>,
    /// Post-market quote.
    pub postmarket_quote: Option<ChartQuote>,
    /// Current market state (e.g. "POST_MARKET").
    pub current_market_state: Option<String>,
}

/// Time series with period and data points.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartTimeSeries {
    /// Time series period (e.g. "ONE_DAY", "ONE_WEEK").
    pub period: String,
    /// Price/volume data points.
    #[serde(default)]
    pub data_points: Vec<ChartDataPoint>,
}

/// Single OHLCV data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataPoint {
    /// Period start timestamp.
    pub start_date_time: String,
    /// Period end timestamp.
    pub end_date_time: String,
    /// Trading volume.
    pub volume: Option<ChartValue>,
    /// Last/close price.
    pub last: Option<ChartValue>,
    /// Period low price.
    pub low: Option<ChartValue>,
    /// Period high price.
    pub high: Option<ChartValue>,
    /// Period open price.
    pub open: Option<ChartValue>,
}

/// Single numeric value.
pub type ChartValue = crate::types::FloatValue;

/// Quote data with formatted display strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartQuote {
    /// Trade timestamp.
    pub trade_date_time: Option<String>,
    /// Data timeliness (e.g. "REAL_TIME", "DELAYED").
    pub timeliness: Option<String>,
    /// Quote type (e.g. "REGULAR", "PRE_MARKET").
    pub quote_type: Option<String>,
    /// Volume with formatted string.
    pub volume: Option<ChartFormattedValue>,
    /// Percent change with formatted string.
    pub percent_change: Option<ChartFormattedValue>,
    /// Net change with formatted string.
    pub net_change: Option<ChartFormattedValue>,
    /// Last price with formatted string.
    pub last: Option<ChartFormattedValue>,
}

/// Numeric value with formatted display string.
pub type ChartFormattedValue = crate::types::FormattedFloat;

/// Exchange information and holidays.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExchangeData {
    /// City name.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub city: Option<String>,
    /// ISO country code.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub country_code: Option<String>,
    /// Exchange ISO code (e.g. "XNYS").
    #[serde(rename = "exchangeISO")]
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub exchange_iso: Option<String>,
    /// Exchange identifier.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub id: Option<String>,
    /// Exchange holidays.
    #[serde(default)]
    pub holidays: Vec<ChartExchangeHoliday>,
}

/// Exchange holiday entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExchangeHoliday {
    /// Holiday name.
    #[serde(deserialize_with = "deserialize_string")]
    pub name: String,
    /// Holiday type (e.g. "FULL").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub holiday_type: Option<String>,
    /// Holiday description.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    /// Start timestamp.
    #[serde(deserialize_with = "deserialize_string")]
    pub start_date_time: String,
    /// End timestamp.
    #[serde(deserialize_with = "deserialize_string")]
    pub end_date_time: String,
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches daily chart market data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    #[allow(clippy::too_many_arguments)]
    pub async fn chart_market_data(
        &self,
        symbols: &[&str],
        symbol_dialect_type: &str,
        start_date_time: &str,
        end_date_time: &str,
        time_series_type: &str,
        include_intraday: bool,
        exchange_name: &str,
    ) -> crate::error::Result<ChartMarketDataResponse> {
        let query = chart_market_data_query(start_date_time, end_date_time)?;
        let variables = ChartMarketDataVariables {
            symbols: symbols_to_owned(symbols),
            symbol_dialect_type: symbol_dialect_type.to_string(),
            filter: TimeSeriesFilterInput {
                start_date_time: EqFilter {
                    eq: start_date_time.to_string(),
                },
                end_date_time: EqFilter {
                    eq: end_date_time.to_string(),
                },
                time_series_type: EqFilter {
                    eq: api_time_series_type(time_series_type).to_string(),
                },
                include_intraday_data: include_intraday,
            },
            exchange_name: exchange_name.to_string(),
        };

        self.graphql_operation("ChartMarketData", variables, query)
            .await
    }

    /// Fetches weekly chart market data for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn chart_market_data_weekly(
        &self,
        symbols: &[&str],
        symbol_dialect_type: &str,
        start_date_time: &str,
        end_date_time: &str,
    ) -> crate::error::Result<ChartMarketDataResponse> {
        let variables = ChartMarketDataWeeklyVariables {
            symbols: symbols_to_owned(symbols),
            symbol_dialect_type: symbol_dialect_type.to_string(),
            filter: TimeSeriesFilterInput {
                start_date_time: EqFilter {
                    eq: start_date_time.to_string(),
                },
                end_date_time: EqFilter {
                    eq: end_date_time.to_string(),
                },
                time_series_type: EqFilter {
                    eq: api_time_series_type("ONE_WEEK").to_string(),
                },
                include_intraday_data: true,
            },
        };

        self.graphql_operation("ChartMarketData", variables, QUERY_CHART_MARKET_DATA_WEEKLY)
            .await
    }
}

fn chart_market_data_query(
    start_date_time: &str,
    end_date_time: &str,
) -> crate::error::Result<String> {
    render_chart_market_data_query(QUERY_CHART_MARKET_DATA, start_date_time, end_date_time)
}

fn render_chart_market_data_query(
    template: &str,
    start_date_time: &str,
    end_date_time: &str,
) -> crate::error::Result<String> {
    ensure_query_placeholder(template, "__START_DATE_TIME__")?;
    ensure_query_placeholder(template, "__END_DATE_TIME__")?;

    let start_date_time = date_time_literal(start_date_time)?;
    let end_date_time = date_time_literal(end_date_time)?;

    Ok(template
        .replace("__START_DATE_TIME__", &start_date_time)
        .replace("__END_DATE_TIME__", &end_date_time))
}

fn ensure_query_placeholder(template: &str, placeholder: &'static str) -> crate::error::Result<()> {
    if template.contains(placeholder) {
        return Ok(());
    }

    Err(serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("chart market data query template missing {placeholder}"),
    ))
    .into())
}

fn date_time_literal(value: &str) -> crate::error::Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn api_time_series_type(value: &str) -> &str {
    match value {
        "ONE_DAY" => "P1D",
        "ONE_WEEK" => "P7D",
        other => other,
    }
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    json_value_to_string(value, DEFAULT_STRING_KEYS)
        .ok_or_else(|| serde::de::Error::custom("expected string-like value"))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{mock_test, mock_test_with_fixture};

    #[tokio::test]
    async fn chart_market_data_parses_response() {
        let (_server, client, mock) = mock_test("ChartMarketData").await;

        let resp = client
            .chart_market_data(
                &["AAPL"],
                "CHARTING",
                "2025-01-01T00:00:00.000Z",
                "2025-05-02T23:59:59.000Z",
                "ONE_DAY",
                true,
                "NYSE",
            )
            .await
            .expect("chart_market_data should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id, "AAPL-CHARTING");

        let origin = item.origin_request.as_ref().expect("origin_request");
        assert_eq!(origin.symbol, "AAPL");
        assert_eq!(origin.from_dialect, "CHARTING");

        let pricing = item.pricing.as_ref().expect("pricing");
        let ts = pricing.time_series.as_ref().expect("time_series");
        assert_eq!(ts.period, "ONE_DAY");
        assert_eq!(ts.data_points.len(), 2);
        assert_eq!(ts.data_points[0].last.as_ref().unwrap().value, Some(210.45));

        let quote = pricing.quote.as_ref().expect("quote");
        assert_eq!(quote.last.as_ref().unwrap().value, Some(212.30));
        assert_eq!(
            quote.last.as_ref().unwrap().formatted_value.as_deref(),
            Some("212.30")
        );

        assert_eq!(pricing.current_market_state.as_deref(), Some("POST_MARKET"));

        let exchange = resp.exchange_data.as_ref().expect("exchange_data");
        assert_eq!(exchange.city.as_deref(), Some("New York"));
        assert_eq!(exchange.exchange_iso.as_deref(), Some("XNYS"));
        assert_eq!(exchange.holidays.len(), 1);
        assert_eq!(exchange.holidays[0].name, "Independence Day");

        mock.assert();
    }

    #[tokio::test]
    async fn chart_market_data_weekly_parses_response() {
        let (_server, client, mock) =
            mock_test_with_fixture("ChartMarketDataWeekly", "ChartMarketData").await;

        let resp = client
            .chart_market_data_weekly(
                &["AAPL"],
                "CHARTING",
                "2024-05-01T00:00:00.000Z",
                "2025-05-02T23:59:59.000Z",
            )
            .await
            .expect("chart_market_data_weekly should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id, "AAPL-CHARTING");

        let pricing = item.pricing.as_ref().expect("pricing");
        let ts = pricing.time_series.as_ref().expect("time_series");
        assert_eq!(ts.period, "ONE_WEEK");
        assert_eq!(ts.data_points.len(), 1);

        assert!(resp.exchange_data.is_none());

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_chart_market_data() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .chart_market_data(
                &["AAPL"],
                "CHARTING",
                "2025-01-01T00:00:00.000Z",
                "2025-05-02T23:59:59.000Z",
                "ONE_DAY",
                true,
                "NYSE",
            )
            .await
            .expect("live chart_market_data should succeed");

        assert!(!resp.market_data.is_empty());
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_chart_market_data_weekly() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .chart_market_data_weekly(
                &["AAPL"],
                "CHARTING",
                "2024-05-01T00:00:00.000Z",
                "2025-05-02T23:59:59.000Z",
            )
            .await
            .expect("live chart_market_data_weekly should succeed");

        assert!(!resp.market_data.is_empty());
    }

    #[test]
    fn chart_market_data_query_inlines_holiday_date_time_literals() {
        let query =
            super::chart_market_data_query("2025-01-01T00:00:00.000Z", "2025-05-02T23:59:59.000Z")
                .expect("query should render");

        assert!(!query.contains("__START_DATE_TIME__"));
        assert!(!query.contains("__END_DATE_TIME__"));
        assert!(!query.contains("$holidayStartDateTime"));
        assert!(query.contains("timeSeries(where: $where)"));
        assert!(query.contains(r#"startDateTime: { gt: "2025-01-01T00:00:00.000Z" }"#));
        assert!(query.contains(r#"endDateTime: { lt: "2025-05-02T23:59:59.000Z" }"#));
    }

    #[test]
    fn chart_market_data_query_reports_missing_holiday_placeholder() {
        let err = super::render_chart_market_data_query(
            "query { holidays(where: { endDateTime: { lt: __END_DATE_TIME__ } }) { name } }",
            "2025-01-01T00:00:00.000Z",
            "2025-05-02T23:59:59.000Z",
        )
        .expect_err("missing placeholder should fail");

        assert!(
            err.to_string()
                .contains("chart market data query template missing __START_DATE_TIME__")
        );
    }

    #[test]
    fn api_time_series_type_maps_legacy_names_to_current_api_periods() {
        assert_eq!(super::api_time_series_type("ONE_DAY"), "P1D");
        assert_eq!(super::api_time_series_type("ONE_WEEK"), "P7D");
        assert_eq!(super::api_time_series_type("P1M"), "P1M");
    }

    #[test]
    fn chart_exchange_holiday_description_accepts_nested_value() {
        let holiday: super::ChartExchangeHoliday = serde_json::from_value(serde_json::json!({
            "name": "Holiday",
            "holidayType": "FULL",
            "description": { "value": "Nested description" },
            "startDateTime": "2025-07-04T00:00:00.000Z",
            "endDateTime": "2025-07-04T23:59:59.000Z"
        }))
        .expect("holiday should deserialize");

        assert_eq!(holiday.description.as_deref(), Some("Nested description"));
    }

    #[test]
    fn chart_exchange_holiday_name_accepts_nested_value() {
        let holiday: super::ChartExchangeHoliday = serde_json::from_value(serde_json::json!({
            "name": { "value": "Nested holiday" },
            "holidayType": "FULL",
            "description": "Description",
            "startDateTime": "2025-07-04T00:00:00.000Z",
            "endDateTime": "2025-07-04T23:59:59.000Z"
        }))
        .expect("holiday should deserialize");

        assert_eq!(holiday.name, "Nested holiday");
    }

    #[test]
    fn chart_exchange_data_accepts_nested_string_values() {
        let exchange: super::ChartExchangeData = serde_json::from_value(serde_json::json!({
            "city": { "value": "New York" },
            "countryCode": { "value": "US" },
            "exchangeISO": { "value": "XNYS" },
            "id": { "value": "NYSE" },
            "holidays": [
                {
                    "name": "Holiday",
                    "holidayType": { "value": "FULL" },
                    "description": "Description",
                    "startDateTime": { "value": "2025-07-04T00:00:00.000Z" },
                    "endDateTime": { "value": "2025-07-04T23:59:59.000Z" }
                }
            ]
        }))
        .expect("exchange data should deserialize");

        assert_eq!(exchange.city.as_deref(), Some("New York"));
        assert_eq!(exchange.country_code.as_deref(), Some("US"));
        assert_eq!(exchange.exchange_iso.as_deref(), Some("XNYS"));
        assert_eq!(exchange.id.as_deref(), Some("NYSE"));
        assert_eq!(exchange.holidays[0].holiday_type.as_deref(), Some("FULL"));
        assert_eq!(
            exchange.holidays[0].start_date_time,
            "2025-07-04T00:00:00.000Z"
        );
        assert_eq!(
            exchange.holidays[0].end_date_time,
            "2025-07-04T23:59:59.000Z"
        );
    }
}
