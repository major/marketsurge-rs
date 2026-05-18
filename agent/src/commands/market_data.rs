//! Broad market data command covering ratings, pricing, industry, and
//! fundamentals for one or more symbols.

use serde::Serialize;
use tracing::instrument;

use crate::cli::SymbolsArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

/// Flat output record for a single symbol's market data snapshot.
///
/// Extracts the most useful fields from the deeply nested
/// `OtherMarketData` response into a single flat row per symbol.
#[derive(Debug, Clone, Serialize)]
pub struct MarketDataRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Company name.
    pub company_name: Option<String>,
    /// Instrument sub-type (e.g. "COMMON_STOCK").
    pub instrument_type: Option<String>,
    /// IPO date.
    pub ipo_date: Option<String>,
    /// Composite rating.
    pub comp_rating: Option<i64>,
    /// Relative Strength rating.
    pub rs_rating: Option<i64>,
    /// EPS rating.
    pub eps_rating: Option<i64>,
    /// SMR rating letter (e.g. "A", "B+").
    pub smr_rating: Option<String>,
    /// Accumulation/Distribution rating letter.
    pub ad_rating: Option<String>,
    /// Market capitalization (formatted).
    pub market_cap: Option<String>,
    /// 50-day average dollar volume (formatted).
    pub avg_dollar_volume_50d: Option<String>,
    /// Up/down volume ratio (formatted).
    pub up_down_volume_ratio: Option<String>,
    /// Short interest as percent of float (formatted).
    pub short_interest_pct_float: Option<String>,
    /// Short interest days to cover (formatted).
    pub short_interest_days_to_cover: Option<String>,
    /// Industry group name.
    pub industry_name: Option<String>,
    /// Industry sector.
    pub industry_sector: Option<String>,
    /// Number of stocks in industry group.
    pub industry_stocks_in_group: Option<i64>,
    /// Fund ownership percent of float (formatted).
    pub funds_pct_float_held: Option<String>,
    /// Next EPS due date.
    pub eps_due_date: Option<String>,
    /// EPS due date status (e.g. "CONFIRMED").
    pub eps_due_date_status: Option<String>,
    /// Debt percent (formatted).
    pub debt_pct: Option<String>,
    /// R&D percent last quarter (formatted).
    pub rd_pct_last_qtr: Option<String>,
}

/// Handles the market-data command.
#[instrument(skip_all)]
pub async fn handle(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .other_market_data(
                    &symbol_refs,
                    "CHARTING",
                    "P12Q_AGO",
                    "P24Q_AGO",
                    "P4Q_FUTURE",
                )
                .await
                .map_err(handle_api_error)?;

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
                let company_name = item
                    .symbology
                    .as_ref()
                    .and_then(|s| s.company.as_ref())
                    .and_then(|c| c.company_name.clone());

                let instrument_type = item
                    .symbology
                    .as_ref()
                    .and_then(|s| s.instrument.as_ref())
                    .and_then(|i| i.sub_type.clone());

                let ipo_date = item
                    .symbology
                    .as_ref()
                    .and_then(|s| s.instrument.as_ref())
                    .and_then(|i| i.ipo_date.as_ref())
                    .and_then(|d| d.value.clone());

                // Ratings: take the first (CURRENT) entry for each.
                let ratings = &item.ratings;
                let comp_rating = ratings
                    .as_ref()
                    .and_then(|r| r.comp_rating.first())
                    .and_then(|r| r.value);
                let rs_rating = ratings
                    .as_ref()
                    .and_then(|r| r.rs_rating.first())
                    .and_then(|r| r.value);
                let eps_rating = ratings
                    .as_ref()
                    .and_then(|r| r.eps_rating.first())
                    .and_then(|r| r.value);
                let smr_rating = ratings
                    .as_ref()
                    .and_then(|r| r.smr_rating.first())
                    .and_then(|r| r.letter_value.clone());
                let ad_rating = ratings
                    .as_ref()
                    .and_then(|r| r.ad_rating.first())
                    .and_then(|r| r.letter_value.clone());

                // Pricing statistics
                let eod = item
                    .pricing_statistics
                    .as_ref()
                    .and_then(|p| p.end_of_day_statistics.as_ref());

                let market_cap = eod
                    .and_then(|e| e.market_capitalization.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                let avg_dollar_volume_50d = eod
                    .and_then(|e| e.avg_dollar_volume_50_day.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                let up_down_volume_ratio = eod
                    .and_then(|e| e.up_down_volume_ratio.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                let short_interest_pct_float = eod
                    .and_then(|e| e.short_interest.as_ref())
                    .and_then(|si| si.percent_of_float.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                let short_interest_days_to_cover = eod
                    .and_then(|e| e.short_interest.as_ref())
                    .and_then(|si| si.days_to_cover.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                // Industry
                let industry = &item.industry;
                let industry_name = industry.as_ref().and_then(|i| i.name.clone());
                let industry_sector = industry.as_ref().and_then(|i| i.sector.clone());
                let industry_stocks_in_group =
                    industry.as_ref().and_then(|i| i.number_of_stocks_in_group);

                // Ownership
                let funds_pct_float_held = item
                    .ownership
                    .as_ref()
                    .and_then(|o| o.funds_float_percent_held.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                // Financials
                let financials = &item.financials;
                let eps_due_date = financials
                    .as_ref()
                    .and_then(|f| f.eps_due_date.as_ref())
                    .and_then(|d| d.formatted_value.clone());
                let eps_due_date_status = financials
                    .as_ref()
                    .and_then(|f| f.eps_due_date_status.clone());

                // Fundamentals
                let fundamentals = &item.fundamentals;
                let debt_pct = fundamentals
                    .as_ref()
                    .and_then(|f| f.debt_percent.as_ref())
                    .and_then(|v| v.formatted_value.clone());
                let rd_pct_last_qtr = fundamentals
                    .as_ref()
                    .and_then(|f| f.research_and_development_percent_last_qtr.as_ref())
                    .and_then(|v| v.formatted_value.clone());

                records.push(MarketDataRecord {
                    symbol: symbol.to_string(),
                    company_name,
                    instrument_type,
                    ipo_date,
                    comp_rating,
                    rs_rating,
                    eps_rating,
                    smr_rating,
                    ad_rating,
                    market_cap,
                    avg_dollar_volume_50d,
                    up_down_volume_ratio,
                    short_interest_pct_float,
                    short_interest_days_to_cover,
                    industry_name,
                    industry_sector,
                    industry_stocks_in_group,
                    funds_pct_float_held,
                    eps_due_date,
                    eps_due_date_status,
                    debt_pct,
                    rd_pct_last_qtr,
                });
            }

            Ok(records)
        },
    )
    .await
}
