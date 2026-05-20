//! Other market data endpoint covering ratings, pricing, patterns, financials,
//! industry, ownership, and fundamentals for one or more symbols.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::Client;
use crate::types::symbols_to_owned;

// ---------------------------------------------------------------------------
// GraphQL query (copied verbatim from Go source; contains {pattern_start_date}
// and {pattern_end_date} placeholders replaced before sending)
// ---------------------------------------------------------------------------

const QUERY_OTHER_MARKET_DATA: &str = include_str!("graphql/other_market_data.graphql");

const PATTERN_LOOKBACK_YEARS: i32 = 4;

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OtherMarketDataVariables {
    symbols: Vec<String>,
    symbol_dialect_type: String,
    up_to_historical_period_for_profit_margin: String,
    up_to_historical_period_offset: String,
    up_to_query_period_offset: String,
}

// ---------------------------------------------------------------------------
// Shared value types
// ---------------------------------------------------------------------------

/// Numeric value with optional formatted display string.
pub type MdFormattedFloat = crate::types::FormattedFloat;

/// Numeric value with scaling factor and formatted display string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdScaledFloat {
    /// Raw numeric value.
    pub value: Option<f64>,
    /// Scaling factor applied to value.
    pub scaling_factor: Option<f64>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
}

/// Single date string value.
pub type MdDateValue = crate::types::DateValue;

/// String value with optional formatted display representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdFormattedString {
    /// Raw string value.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub value: Option<String>,
    /// Display-formatted string.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub formatted_value: Option<String>,
}

/// Wrapper for a single nested numeric value (`{ "value": ... }`).
pub type MdValueWrapper = crate::types::FloatValue;

/// Currency symbol formatting information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdCurrencySymbolInfo {
    /// Decimal precision for the mantissa.
    pub mantissa_precision: Option<i64>,
    /// Currency unit symbol (e.g. "$").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub unit_symbol: Option<String>,
    /// ISO 4217 currency code (e.g. "USD").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub iso_currency_code: Option<String>,
    /// Whether the symbol appears after the value.
    pub is_suffix: Option<bool>,
}

/// Currency value with symbol information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdCurrencyValue {
    /// Raw numeric value.
    pub value: Option<f64>,
    /// Scaling factor applied to value.
    pub scaling_factor: Option<f64>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
    /// Currency symbol formatting.
    pub currency_symbol_info: Option<MdCurrencySymbolInfo>,
}

// ---------------------------------------------------------------------------
// Top-level response
// ---------------------------------------------------------------------------

/// Top-level response from the `OtherMarketData` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherMarketDataResponse {
    /// Per-symbol market data items.
    #[serde(default, deserialize_with = "deserialize_market_data_items")]
    pub market_data: Vec<MdMarketDataItem>,
}

fn deserialize_market_data_items<'de, D>(deserializer: D) -> Result<Vec<MdMarketDataItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<Value>::deserialize(deserializer)?;

    Ok(values
        .into_iter()
        .map(|value| {
            serde_json::from_value(value.clone()).unwrap_or_else(|error| {
                let mut item = empty_market_data_item();
                item.id = value.get("id").cloned().and_then(json_value_to_string);
                item.origin_request = value
                    .get("originRequest")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok());
                item.decode_error = Some(error.to_string());
                item
            })
        })
        .collect())
}

fn empty_market_data_item() -> MdMarketDataItem {
    MdMarketDataItem {
        id: None,
        origin_request: None,
        ratings: None,
        pricing_statistics: None,
        corporate_actions: None,
        symbology: None,
        pattern_info: None,
        financials: None,
        industry: None,
        ownership: None,
        fundamentals: None,
        decode_error: None,
    }
}

/// Market data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdMarketDataItem {
    /// Symbol identifier.
    pub id: Option<String>,
    /// Original request details.
    pub origin_request: Option<MdOriginRequest>,
    /// IBD ratings (Composite, RS, EPS, SMR, A/D).
    pub ratings: Option<MdRatings>,
    /// End-of-day and intraday pricing statistics.
    pub pricing_statistics: Option<MdPricingStatistics>,
    /// Dividend, split, and spinoff history.
    pub corporate_actions: Option<MdCorporateActions>,
    /// Company and instrument symbology.
    pub symbology: Option<MdSymbology>,
    /// Chart pattern recognition data.
    pub pattern_info: Option<MdPatternInfo>,
    /// Earnings, sales, margins, and estimates.
    pub financials: Option<MdFinancials>,
    /// Industry group information.
    pub industry: Option<MdIndustry>,
    /// Fund ownership metrics.
    pub ownership: Option<MdOwnership>,
    /// Fundamental financial data.
    pub fundamentals: Option<MdFundamentals>,
    /// Per-item decode error captured when a row has an unsupported shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decode_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Origin request
// ---------------------------------------------------------------------------

/// Original symbol request details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdOriginRequest {
    /// Symbol dialect used (e.g. "CHARTING").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub from_dialect: Option<String>,
    /// Requested symbol.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub symbol: Option<String>,
}

// ---------------------------------------------------------------------------
// Ratings
// ---------------------------------------------------------------------------

/// IBD ratings for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdRatings {
    /// Composite rating.
    #[serde(default)]
    pub comp_rating: Vec<MdRating>,
    /// Relative Strength rating.
    #[serde(default)]
    pub rs_rating: Vec<MdRating>,
    /// EPS rating.
    #[serde(default)]
    pub eps_rating: Vec<MdRating>,
    /// SMR rating.
    #[serde(default)]
    pub smr_rating: Vec<MdRating>,
    /// Accumulation/Distribution rating.
    #[serde(default)]
    pub ad_rating: Vec<MdRating>,
}

/// Single rating value with period metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdRating {
    /// Numeric rating value.
    pub value: Option<i64>,
    /// Period offset (e.g. "CURRENT").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub period_offset: Option<String>,
    /// Period identifier (e.g. "P12M").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub period: Option<String>,
    /// Letter grade value (e.g. "A", "B+").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub letter_value: Option<String>,
}

// ---------------------------------------------------------------------------
// Pricing statistics
// ---------------------------------------------------------------------------

/// End-of-day and intraday pricing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdPricingStatistics {
    /// End-of-day statistics.
    pub end_of_day_statistics: Option<MdEndOfDayStatistics>,
    /// Intraday statistics.
    pub intraday_statistics: Option<MdIntradayStatistics>,
}

/// End-of-day pricing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdEndOfDayStatistics {
    /// Historical price statistics per period.
    #[serde(default)]
    pub historical_price_statistics: Vec<MdHistoricalPriceStatistic>,
    /// Earliest available pricing date.
    pub pricing_start_date: Option<MdDateValue>,
    /// Latest available pricing date.
    pub pricing_end_date: Option<MdDateValue>,
    /// Volume moving averages.
    #[serde(default)]
    pub volume_moving_averages: Vec<MdVolumeMovingAverage>,
    /// 50-day average dollar volume.
    pub avg_dollar_volume_50_day: Option<MdFormattedFloat>,
    /// Market capitalization.
    pub market_capitalization: Option<MdFormattedFloat>,
    /// Average true range percent.
    #[serde(default)]
    pub average_true_range_percent: Vec<MdAverageTrueRangePercent>,
    /// Anticipated events dates.
    #[serde(default)]
    pub ant_events: Vec<MdDateValue>,
    /// Up/down volume ratio.
    pub up_down_volume_ratio: Option<MdScaledFloat>,
    /// Alpha statistic.
    pub alpha: Option<MdScaledFloat>,
    /// Beta statistic.
    pub beta: Option<MdScaledFloat>,
    /// Short interest data.
    pub short_interest: Option<MdShortInterest>,
    /// Daily blue dot event dates.
    #[serde(default)]
    pub blue_dot_daily_events: Vec<MdFormattedString>,
    /// Weekly blue dot event dates.
    #[serde(default)]
    pub blue_dot_weekly_events: Vec<MdFormattedString>,
}

/// Historical price statistics for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdHistoricalPriceStatistic {
    /// Period identifier (e.g. "P1Q").
    pub period: Option<String>,
    /// Period offset (e.g. "CURRENT").
    pub period_offset: Option<String>,
    /// Period end date.
    pub period_end_date: Option<MdFormattedString>,
    /// Date of period high price.
    pub price_high_date: Option<MdFormattedString>,
    /// Period high price.
    pub price_high: Option<MdFormattedFloat>,
    /// Date of period low price.
    pub price_low_date: Option<MdFormattedString>,
    /// Period low price.
    pub price_low: Option<MdFormattedFloat>,
    /// Period closing price.
    pub price_close: Option<MdFormattedFloat>,
    /// Price percent change over the period.
    pub price_percent_change: Option<MdFormattedFloat>,
}

/// Volume moving average with period metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdVolumeMovingAverage {
    /// Moving average volume value.
    pub value: Option<f64>,
    /// Period identifier (e.g. "P50D").
    pub period: Option<String>,
    /// Period offset (e.g. "CURRENT").
    pub period_offset: Option<String>,
}

/// Average true range percent with period metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdAverageTrueRangePercent {
    /// ATR percent value.
    pub value: Option<f64>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
    /// Period identifier (e.g. "P21D").
    pub period: Option<String>,
    /// Period offset.
    pub period_offset: Option<String>,
}

/// Short interest metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdShortInterest {
    /// Days to cover.
    pub days_to_cover: Option<MdFormattedFloat>,
    /// Days to cover percent change.
    pub days_to_cover_percent_change: Option<MdFormattedFloat>,
    /// Short interest as percent of float.
    pub percent_of_float: Option<MdScaledFloat>,
    /// Short interest volume.
    pub volume: Option<MdScaledFloat>,
}

/// Intraday pricing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdIntradayStatistics {
    /// Price percent change vs. a subject.
    #[serde(default)]
    pub price_percent_change_vs: Vec<MdPercentChangeVs>,
    /// Volume percent change vs. a subject.
    #[serde(default)]
    pub volume_percent_change_vs: Vec<MdPercentChangeVs>,
    /// Whether today is a daily blue dot event.
    pub is_daily_blue_dot_event: Option<bool>,
    /// Whether today is a weekly blue dot event.
    pub is_weekly_blue_dot_event: Option<bool>,
    /// Dividend yield.
    #[serde(rename = "yield")]
    pub yield_value: Option<MdScaledFloat>,
    /// Price-to-cash-flow ratio.
    pub price_to_cash_flow_ratio: Option<MdScaledFloat>,
    /// Forward price-to-earnings ratio.
    pub forward_price_to_earnings_ratio: Option<MdScaledFloat>,
    /// Price-to-sales ratio.
    pub price_to_sales_ratio: Option<MdScaledFloat>,
    /// Price-to-earnings ratio.
    pub price_to_earnings_ratio: Option<MdScaledFloat>,
    /// Price-to-earnings ratio vs. S&P 500.
    #[serde(rename = "priceToEarningsVsSP500")]
    pub price_to_earnings_vs_sp500: Option<MdScaledFloat>,
}

/// Percent change relative to a subject and period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdPercentChangeVs {
    /// Percent change value.
    pub value: Option<f64>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
    /// Comparison subject (e.g. "PREVIOUS_CLOSE", "P50D_AVG").
    pub subject: Option<String>,
    /// Period identifier.
    pub period: Option<String>,
}

// ---------------------------------------------------------------------------
// Corporate actions
// ---------------------------------------------------------------------------

/// Dividend, split, and spinoff history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdCorporateActions {
    /// Next reported ex-dividend date.
    pub dividend_next_reported_ex_date: Option<MdFormattedString>,
    /// Dividend history.
    #[serde(default)]
    pub dividends: Vec<MdDividend>,
    /// Spinoff history.
    #[serde(default)]
    pub spinoffs: Vec<MdSpinoff>,
    /// Stock split history.
    #[serde(default)]
    pub splits: Vec<MdSplit>,
}

/// Single dividend event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdDividend {
    /// Dividend amount.
    pub amount: Option<MdFormattedFloat>,
    /// Change direction indicator (e.g. "UNCHANGED").
    pub change_indicator: Option<String>,
    /// Ex-dividend date.
    pub ex_date: Option<MdDateValue>,
}

/// Single spinoff event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdSpinoff {
    /// Ex-date for the spinoff.
    pub ex_date: Option<MdDateValue>,
}

/// Single stock split event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdSplit {
    /// Split date.
    pub split_date: Option<MdDateValue>,
}

// ---------------------------------------------------------------------------
// Symbology
// ---------------------------------------------------------------------------

/// Company and instrument symbology.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdSymbology {
    /// Company profile.
    pub company: Option<MdCompany>,
    /// Instrument metadata.
    pub instrument: Option<MdInstrument>,
}

/// Company profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdCompany {
    /// Company name.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub company_name: Option<String>,
    /// Primary address line.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub address: Option<String>,
    /// Secondary address line.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub address2: Option<String>,
    /// Phone number.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub phone: Option<String>,
    /// Business description.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub business_description: Option<String>,
    /// Company website URL.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub url: Option<String>,
    /// City.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub city: Option<String>,
    /// Country code.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub country: Option<String>,
    /// State or province.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub state_province: Option<String>,
}

/// Instrument metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdInstrument {
    /// Instrument sub-type (e.g. "COMMON_STOCK").
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub sub_type: Option<String>,
    /// IPO date.
    pub ipo_date: Option<MdDateValue>,
    /// IPO price with currency information.
    pub ipo_price: Option<MdCurrencyValue>,
}

// ---------------------------------------------------------------------------
// Pattern info
// ---------------------------------------------------------------------------

/// Chart pattern recognition data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdPatternInfo {
    /// Detected chart patterns (union of all pattern types).
    #[serde(default)]
    pub patterns: Vec<MdPattern>,
    /// Tight consolidation areas.
    #[serde(default)]
    pub tight_areas: Vec<MdTightArea>,
}

/// Flat union of all chart pattern types. Fields specific to a particular
/// pattern type are `None` when the pattern is a different type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct MdPattern {
    // Common fields present in most pattern types.
    pub id: Option<String>,
    pub pattern_type: Option<String>,
    pub periodicity: Option<String>,
    pub base_stage: Option<String>,
    pub base_number: Option<i64>,
    pub base_status: Option<String>,
    pub base_length: Option<i64>,
    pub base_depth: Option<MdScaledFloat>,
    pub base_start_date: Option<MdDateValue>,
    pub base_end_date: Option<MdDateValue>,
    pub base_bottom_date: Option<MdDateValue>,
    pub left_side_high_date: Option<MdDateValue>,
    pub pivot_price: Option<MdCurrencyValue>,
    pub pivot_date: Option<MdDateValue>,
    pub pivot_price_date: Option<MdDateValue>,
    pub avg_volume_rate_pct_on_pivot: Option<MdScaledFloat>,
    pub price_pct_change_on_pivot: Option<MdScaledFloat>,

    // Cup/Saucer-specific fields.
    pub handle_depth: Option<MdScaledFloat>,
    pub handle_length: Option<i64>,
    pub cup_length: Option<i64>,
    pub cup_end_date: Option<MdDateValue>,
    pub handle_low_date: Option<MdDateValue>,
    pub handle_start_date: Option<MdDateValue>,

    // IPO base-specific fields.
    pub up_bars: Option<i64>,
    pub blue_bars: Option<i64>,
    pub stall_bars: Option<i64>,
    pub down_bars: Option<i64>,
    pub red_bars: Option<i64>,
    pub support_bars: Option<i64>,
    pub up_volume_total: Option<MdScaledFloat>,
    pub down_volume_total: Option<MdScaledFloat>,
    pub volume_pct_change_on_pivot: Option<MdScaledFloat>,

    // Ascending base-specific fields.
    pub first_bottom_date: Option<MdDateValue>,
    pub second_ascending_high_date: Option<MdDateValue>,
    pub second_bottom_date: Option<MdDateValue>,
    pub third_ascending_high_date: Option<MdDateValue>,
    pub third_bottom_date: Option<MdDateValue>,
    pub pull_back_1_depth: Option<MdScaledFloat>,
    pub pull_back_2_depth: Option<MdScaledFloat>,
    pub pull_back_3_depth: Option<MdScaledFloat>,

    // Double bottom-specific fields.
    pub mid_peak_date: Option<MdDateValue>,
}

/// Tight price consolidation area on a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdTightArea {
    /// Associated pattern identifier.
    #[serde(rename = "patternID")]
    pub pattern_id: Option<i64>,
    /// Consolidation start date.
    pub start_date: Option<MdDateValue>,
    /// Consolidation end date.
    pub end_date: Option<MdDateValue>,
    /// Number of trading days.
    pub length: Option<i64>,
}

// ---------------------------------------------------------------------------
// Financials
// ---------------------------------------------------------------------------

/// Earnings, sales, margins, and estimate data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdFinancials {
    /// Next EPS report due date.
    pub eps_due_date: Option<MdFormattedString>,
    /// EPS due date status (e.g. "CONFIRMED").
    pub eps_due_date_status: Option<String>,
    /// Last EPS report date.
    pub eps_last_reported_date: Option<MdDateValue>,
    /// Consensus EPS and sales data.
    pub consensus_financials: Option<MdConsensusFinancials>,
    /// Cash flow per share for the last year.
    pub cash_flow_per_share_last_year: Option<MdFormattedFloat>,
    /// Profit margin values across periods.
    #[serde(default)]
    pub profit_margin_values: Vec<MdProfitMarginValue>,
    /// Forward EPS and sales estimates.
    pub estimates: Option<MdEstimates>,
}

/// Consensus EPS and sales data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdConsensusFinancials {
    /// Consensus EPS data.
    pub eps: Option<MdConsensusEps>,
    /// Consensus sales data.
    pub sales: Option<MdConsensusSales>,
}

/// Consensus EPS data with reported earnings and growth rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdConsensusEps {
    /// Reported earnings periods.
    #[serde(default)]
    pub reported_earnings: Vec<MdReportedPeriod>,
    /// EPS growth rate.
    #[serde(default)]
    pub growth_rate: Vec<MdGrowthRate>,
    /// Earnings stability score.
    pub earnings_stability: Option<i64>,
}

/// Consensus sales data with reported sales and growth rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdConsensusSales {
    /// Reported sales periods.
    #[serde(default)]
    pub reported_sales: Vec<MdReportedPeriod>,
    /// Sales growth rate.
    #[serde(default)]
    pub growth_rate: Vec<MdGrowthRate>,
}

/// Single reported earnings or sales period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdReportedPeriod {
    /// Numeric value.
    pub value: Option<MdValueWrapper>,
    /// Year-over-year percent change.
    #[serde(rename = "percentChangeYOY")]
    pub percent_change_yoy: Option<MdValueWrapper>,
    /// Period offset (e.g. "CURRENT").
    pub period_offset: Option<String>,
    /// Period identifier (e.g. "P1Q").
    pub period: Option<String>,
    /// Period end date.
    pub period_end_date: Option<MdDateValue>,
    /// Effective date.
    pub effective_date: Option<MdDateValue>,
    /// Percent surprise vs. estimates.
    pub percent_surprise: Option<MdValueWrapper>,
    /// Surprise amount vs. estimates.
    pub surprise_amount: Option<MdValueWrapper>,
    /// Fiscal quarter number.
    pub quarter_number: Option<i64>,
    /// Fiscal year.
    pub fiscal_year: Option<i64>,
}

/// Growth rate with scaling and period metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdGrowthRate {
    /// Growth rate value.
    pub value: Option<f64>,
    /// Scaling factor applied to value.
    pub scaling_factor: Option<f64>,
    /// Period identifier (e.g. "P1Y").
    pub period: Option<String>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
}

/// Profit margin metrics for a single period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdProfitMarginValue {
    /// Period identifier (e.g. "P1Q").
    pub period: Option<String>,
    /// Pre-tax profit margin.
    pub pre_tax_margin: Option<MdScaledFloat>,
    /// After-tax profit margin.
    pub after_tax_margin: Option<MdValueWrapper>,
    /// Gross profit margin.
    pub gross_margin: Option<MdValueWrapper>,
    /// Return on equity.
    pub return_on_equity: Option<MdFormattedFloat>,
    /// Period end date.
    pub period_end_date: Option<MdFormattedString>,
    /// Period offset (e.g. "CURRENT").
    pub period_offset: Option<String>,
}

/// Forward EPS and sales estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdEstimates {
    /// Forward EPS estimates.
    #[serde(default)]
    pub eps_estimates: Vec<MdEstimate>,
    /// Forward sales estimates.
    #[serde(default)]
    pub sales_estimates: Vec<MdEstimate>,
}

/// Single forward earnings or sales estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdEstimate {
    /// Estimate revision direction (e.g. "UP", "DOWN", "FLAT").
    pub revision_direction: Option<String>,
    /// Effective date.
    pub effective_date: Option<MdDateValue>,
    /// Period identifier (e.g. "P1Q").
    pub period: Option<String>,
    /// Estimate value.
    pub value: Option<MdValueWrapper>,
    /// Year-over-year percent change.
    #[serde(rename = "percentChangeYOY")]
    pub percent_change_yoy: Option<MdValueWrapper>,
    /// Period end date.
    pub period_end_date: Option<MdDateValue>,
    /// Estimate type (e.g. "CONSENSUS").
    #[serde(rename = "type")]
    pub estimate_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Industry
// ---------------------------------------------------------------------------

/// Industry group information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdIndustry {
    /// Industry group name.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub name: Option<String>,
    /// Sector name.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub sector: Option<String>,
    /// Industry group code.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub ind_code: Option<String>,
    /// Industry group rank values.
    #[serde(default)]
    pub group_ranks: Vec<MdGroupRank>,
    /// Industry group relative strength.
    #[serde(default, rename = "groupRS")]
    pub group_rs: Vec<MdGroupRs>,
    /// Number of stocks in the industry group.
    pub number_of_stocks_in_group: Option<i64>,
}

/// Industry group rank for a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdGroupRank {
    /// Rank value.
    pub value: Option<i64>,
    /// Period identifier.
    pub period: Option<String>,
    /// Period offset.
    pub period_offset: Option<String>,
}

/// Industry group relative strength for a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdGroupRs {
    /// RS value.
    pub value: Option<i64>,
    /// Period offset.
    pub period_offset: Option<String>,
    /// Letter grade.
    pub letter_value: Option<String>,
    /// Period identifier.
    pub period: Option<String>,
}

// ---------------------------------------------------------------------------
// Ownership
// ---------------------------------------------------------------------------

/// Fund ownership metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdOwnership {
    /// Percent of float held by funds.
    pub funds_float_percent_held: Option<MdScaledFloat>,
}

// ---------------------------------------------------------------------------
// Fundamentals
// ---------------------------------------------------------------------------

/// Fundamental financial data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdFundamentals {
    /// R&D spending as percent of revenue (last quarter).
    pub research_and_development_percent_last_qtr: Option<MdScaledFloat>,
    /// Date of most recent CEO appointment.
    #[serde(rename = "newCEODate")]
    pub new_ceo_date: Option<MdDateValue>,
    /// Debt as percent of capital.
    pub debt_percent: Option<MdFormattedFloat>,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;

    Ok(value.and_then(json_value_to_string))
}

fn json_value_to_string(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Array(mut values) => match values.len() {
            0 => None,
            1 => values.pop().and_then(json_value_to_string),
            _ => Some(Value::Array(values).to_string()),
        },
        Value::Object(map) => {
            for key in ["value", "formattedValue", "displayValue", "name"] {
                if let Some(value) = map.get(key).cloned().and_then(json_value_to_string) {
                    return Some(value);
                }
            }

            Some(Value::Object(map).to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches other market data (ratings, pricing, patterns, financials,
    /// industry, ownership, and fundamentals) for the given symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn other_market_data(
        &self,
        symbols: &[&str],
        symbol_dialect_type: &str,
        historical_period_profit_margin: &str,
        historical_period_offset: &str,
        query_period_offset: &str,
    ) -> crate::error::Result<OtherMarketDataResponse> {
        let now = Utc::now();
        let pattern_end = now.format("%Y-%m-%d").to_string();
        let pattern_start = now
            .checked_sub_months(chrono::Months::new(
                u32::try_from(PATTERN_LOOKBACK_YEARS).unwrap_or(4) * 12,
            ))
            .unwrap_or(now)
            .format("%Y-%m-%d")
            .to_string();

        let query = QUERY_OTHER_MARKET_DATA
            .replace("{pattern_start_date}", &pattern_start)
            .replace("{pattern_end_date}", &pattern_end);

        let variables = OtherMarketDataVariables {
            symbols: symbols_to_owned(symbols),
            symbol_dialect_type: symbol_dialect_type.to_string(),
            up_to_historical_period_for_profit_margin: historical_period_profit_margin.to_string(),
            up_to_historical_period_offset: historical_period_offset.to_string(),
            up_to_query_period_offset: query_period_offset.to_string(),
        };

        self.graphql_operation("OtherMarketData", variables, query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{load_fixture, mock_test};

    #[tokio::test]
    async fn other_market_data_parses_response() {
        let (_server, client, mock) = mock_test("OtherMarketData").await;

        let resp = client
            .other_market_data(&["AAPL"], "CHARTING", "P12Q_AGO", "P24Q_AGO", "P4Q_FUTURE")
            .await
            .expect("other_market_data should succeed");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert_eq!(item.id.as_deref(), Some("AAPL"));

        // Origin request
        let origin = item.origin_request.as_ref().expect("origin_request");
        assert_eq!(origin.from_dialect.as_deref(), Some("CHARTING"));
        assert_eq!(origin.symbol.as_deref(), Some("AAPL"));

        // Ratings
        let ratings = item.ratings.as_ref().expect("ratings");
        assert_eq!(ratings.comp_rating.len(), 1);
        assert_eq!(ratings.comp_rating[0].value, Some(95));
        assert_eq!(ratings.rs_rating.len(), 1);
        assert_eq!(ratings.rs_rating[0].value, Some(92));
        assert_eq!(ratings.smr_rating[0].letter_value.as_deref(), Some("A"));
        assert_eq!(ratings.ad_rating[0].letter_value.as_deref(), Some("B+"));

        // Pricing statistics - end of day
        let pricing = item
            .pricing_statistics
            .as_ref()
            .expect("pricing_statistics");
        let eod = pricing
            .end_of_day_statistics
            .as_ref()
            .expect("end_of_day_statistics");
        assert_eq!(eod.historical_price_statistics.len(), 1);
        assert_eq!(
            eod.historical_price_statistics[0]
                .price_high
                .as_ref()
                .unwrap()
                .value,
            Some(198.5)
        );
        assert_eq!(
            eod.market_capitalization.as_ref().unwrap().value,
            Some(3_200_000_000_000.0)
        );
        let si = eod.short_interest.as_ref().expect("short_interest");
        assert_eq!(si.days_to_cover.as_ref().unwrap().value, Some(1.5));

        // Pricing statistics - intraday
        let intraday = pricing
            .intraday_statistics
            .as_ref()
            .expect("intraday_statistics");
        assert_eq!(intraday.is_daily_blue_dot_event, Some(true));
        assert_eq!(intraday.is_weekly_blue_dot_event, Some(false));
        assert_eq!(
            intraday.price_to_earnings_vs_sp500.as_ref().unwrap().value,
            Some(1.45)
        );
        assert_eq!(intraday.yield_value.as_ref().unwrap().value, Some(0.55));

        // Corporate actions
        let corp = item.corporate_actions.as_ref().expect("corporate_actions");
        assert_eq!(corp.dividends.len(), 1);
        assert_eq!(
            corp.dividends[0].change_indicator.as_deref(),
            Some("UNCHANGED")
        );
        assert!(corp.spinoffs.is_empty());
        assert_eq!(corp.splits.len(), 1);

        // Symbology
        let symb = item.symbology.as_ref().expect("symbology");
        let company = symb.company.as_ref().expect("company");
        assert_eq!(company.company_name.as_deref(), Some("Apple Inc."));
        assert_eq!(company.city.as_deref(), Some("Cupertino"));
        let instrument = symb.instrument.as_ref().expect("instrument");
        assert_eq!(instrument.sub_type.as_deref(), Some("COMMON_STOCK"));

        // Pattern info
        let pattern_info = item.pattern_info.as_ref().expect("pattern_info");
        assert_eq!(pattern_info.patterns.len(), 1);
        let pattern = &pattern_info.patterns[0];
        assert_eq!(pattern.pattern_type.as_deref(), Some("CUP_WITH_HANDLE"));
        assert_eq!(pattern.pivot_price.as_ref().unwrap().value, Some(195.5));
        assert_eq!(pattern_info.tight_areas.len(), 1);
        assert_eq!(pattern_info.tight_areas[0].pattern_id, Some(1));

        // Financials
        let fin = item.financials.as_ref().expect("financials");
        assert_eq!(fin.eps_due_date_status.as_deref(), Some("CONFIRMED"));
        let consensus = fin
            .consensus_financials
            .as_ref()
            .expect("consensus_financials");
        let eps = consensus.eps.as_ref().expect("eps");
        assert_eq!(eps.reported_earnings.len(), 1);
        assert_eq!(
            eps.reported_earnings[0].value.as_ref().unwrap().value,
            Some(1.65)
        );
        assert_eq!(eps.earnings_stability, Some(3));
        let sales = consensus.sales.as_ref().expect("sales");
        assert_eq!(sales.reported_sales.len(), 1);
        assert_eq!(fin.profit_margin_values.len(), 1);
        assert_eq!(
            fin.profit_margin_values[0]
                .pre_tax_margin
                .as_ref()
                .unwrap()
                .value,
            Some(30.5)
        );
        let estimates = fin.estimates.as_ref().expect("estimates");
        assert_eq!(estimates.eps_estimates.len(), 1);
        assert_eq!(
            estimates.eps_estimates[0].revision_direction.as_deref(),
            Some("UP")
        );
        assert_eq!(
            estimates.eps_estimates[0].estimate_type.as_deref(),
            Some("CONSENSUS")
        );

        // Industry
        let industry = item.industry.as_ref().expect("industry");
        assert_eq!(industry.name.as_deref(), Some("Comp-Hardware/Peripherals"));
        assert_eq!(industry.sector.as_deref(), Some("Technology"));
        assert_eq!(industry.number_of_stocks_in_group, Some(25));
        assert_eq!(industry.group_rs.len(), 1);
        assert_eq!(industry.group_rs[0].letter_value.as_deref(), Some("A"));

        // Ownership
        let ownership = item.ownership.as_ref().expect("ownership");
        assert_eq!(
            ownership.funds_float_percent_held.as_ref().unwrap().value,
            Some(62.5)
        );

        // Fundamentals
        let fundamentals = item.fundamentals.as_ref().expect("fundamentals");
        assert_eq!(
            fundamentals.new_ceo_date.as_ref().unwrap().value.as_deref(),
            Some("2011-08-24")
        );

        mock.assert();
    }

    #[test]
    fn other_market_data_accepts_mixed_string_shapes() {
        let fixture = load_fixture("OtherMarketDataMixedShape", "response.json");
        let resp: super::OtherMarketDataResponse =
            serde_json::from_str(&fixture).expect("mixed string shapes should parse");

        assert_eq!(resp.market_data.len(), 1);
        let item = &resp.market_data[0];
        assert!(item.decode_error.is_none());
        assert_eq!(
            item.origin_request
                .as_ref()
                .and_then(|origin| origin.symbol.as_deref()),
            Some("APLD")
        );
        assert_eq!(
            item.ratings
                .as_ref()
                .and_then(|ratings| ratings.smr_rating.first())
                .and_then(|rating| rating.letter_value.as_deref()),
            Some("A")
        );
        assert_eq!(
            item.symbology
                .as_ref()
                .and_then(|symbology| symbology.company.as_ref())
                .and_then(|company| company.company_name.as_deref()),
            Some("Applied Digital Corp")
        );
        assert_eq!(
            item.industry
                .as_ref()
                .and_then(|industry| industry.ind_code.as_deref()),
            Some("G3620")
        );
    }

    #[test]
    fn other_market_data_keeps_rows_when_one_item_fails_to_decode() {
        let resp: super::OtherMarketDataResponse = serde_json::from_value(serde_json::json!({
            "marketData": [
                {"id": "GOOD", "originRequest": {"symbol": "GOOD"}},
                {
                    "id": "BAD",
                    "originRequest": {"symbol": "BAD"},
                    "ratings": {"compRating": {"value": 99}}
                }
            ]
        }))
        .expect("per-item decode failures should not fail whole response");

        assert_eq!(resp.market_data.len(), 2);
        assert_eq!(resp.market_data[0].id.as_deref(), Some("GOOD"));
        assert!(resp.market_data[0].decode_error.is_none());
        assert_eq!(resp.market_data[1].id.as_deref(), Some("BAD"));
        assert_eq!(
            resp.market_data[1]
                .origin_request
                .as_ref()
                .and_then(|origin| origin.symbol.as_deref()),
            Some("BAD")
        );
        assert!(
            resp.market_data[1]
                .decode_error
                .as_deref()
                .is_some_and(|error| error.contains("invalid type"))
        );
    }

    #[test]
    fn json_value_to_string_handles_array_and_object_fallbacks() {
        assert_eq!(super::json_value_to_string(serde_json::json!(null)), None);
        assert_eq!(super::json_value_to_string(serde_json::json!([])), None);
        assert_eq!(
            super::json_value_to_string(serde_json::json!(["A", "B"])).as_deref(),
            Some("[\"A\",\"B\"]")
        );
        assert_eq!(
            super::json_value_to_string(serde_json::json!({"unexpected": 1})).as_deref(),
            Some("{\"unexpected\":1}")
        );
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_other_market_data() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .other_market_data(&["AAPL"], "CHARTING", "P12Q_AGO", "P24Q_AGO", "P4Q_FUTURE")
            .await
            .expect("live other_market_data should succeed");

        assert!(!resp.market_data.is_empty());
    }
}
