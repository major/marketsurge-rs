//! Broad market data command covering ratings, pricing, industry, and
//! fundamentals for one or more symbols.

use serde::Serialize;
use tracing::instrument;

use crate::cli::SymbolsArgs;
use marketsurge_client::market_data::MdMarketDataItem;

use crate::common::command::{api_call, run_command, zip_symbols};

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
    /// Per-symbol market-data decode error, when only this row failed.
    pub decode_error: Option<String>,
}

/// Flattens nested [`MdMarketDataItem`] responses into flat
/// [`MarketDataRecord`] rows, one per symbol.
fn flatten_market_data(
    symbols: &[&str],
    market_data: &[MdMarketDataItem],
) -> Vec<MarketDataRecord> {
    let mut records = Vec::new();

    for (symbol, item) in zip_symbols(symbols, market_data) {
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
        let industry_stocks_in_group = industry.as_ref().and_then(|i| i.number_of_stocks_in_group);

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
            decode_error: item.decode_error.clone(),
        });
    }

    records
}

fn all_rows_failed_to_decode(records: &[MarketDataRecord]) -> bool {
    !records.is_empty() && records.iter().all(|record| record.decode_error.is_some())
}

/// Handles the market-data command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &SymbolsArgs, fields: &[String]) -> i32 {
    run_command(&args.symbols, fields, |client, symbol_refs| async move {
        let response = api_call(client.other_market_data(
            &symbol_refs,
            "CHARTING",
            "P12Q_AGO",
            "P24Q_AGO",
            "P4Q_FUTURE",
        ))
        .await?;

        let records = flatten_market_data(&symbol_refs, &response.market_data);
        if all_rows_failed_to_decode(&records) {
            for record in &records {
                if let Some(error) = &record.decode_error {
                    eprintln!(
                        "market-data decode error for {}: {error}",
                        record.symbol.as_str()
                    );
                }
            }
            return Err(1);
        }

        Ok(records)
    })
    .await
}

#[cfg(test)]
mod tests {
    use marketsurge_client::market_data::{
        MdCompany, MdEndOfDayStatistics, MdFinancials, MdFormattedString, MdFundamentals,
        MdIndustry, MdInstrument, MdMarketDataItem, MdOwnership, MdPricingStatistics, MdRating,
        MdRatings, MdScaledFloat, MdShortInterest, MdSymbology,
    };
    use marketsurge_client::types::{DateValue, FormattedFloat};

    use super::*;

    /// Build an `MdMarketDataItem` with all fields set to `None`/empty.
    fn empty_item() -> MdMarketDataItem {
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

    #[test]
    fn flatten_happy_path() {
        let item = MdMarketDataItem {
            symbology: Some(MdSymbology {
                company: Some(MdCompany {
                    company_name: Some("Apple Inc".to_string()),
                    address: None,
                    address2: None,
                    phone: None,
                    business_description: None,
                    url: None,
                    city: None,
                    country: None,
                    state_province: None,
                }),
                instrument: Some(MdInstrument {
                    sub_type: Some("COMMON_STOCK".to_string()),
                    ipo_date: Some(DateValue {
                        value: Some("1980-12-12".to_string()),
                    }),
                    ipo_price: None,
                }),
            }),
            ratings: Some(MdRatings {
                comp_rating: vec![MdRating {
                    value: Some(95),
                    period_offset: None,
                    period: None,
                    letter_value: None,
                }],
                rs_rating: vec![MdRating {
                    value: Some(92),
                    period_offset: None,
                    period: None,
                    letter_value: None,
                }],
                eps_rating: vec![MdRating {
                    value: Some(88),
                    period_offset: None,
                    period: None,
                    letter_value: None,
                }],
                smr_rating: vec![MdRating {
                    value: None,
                    period_offset: None,
                    period: None,
                    letter_value: Some("A".to_string()),
                }],
                ad_rating: vec![MdRating {
                    value: None,
                    period_offset: None,
                    period: None,
                    letter_value: Some("B+".to_string()),
                }],
            }),
            industry: Some(MdIndustry {
                name: Some("Computers-Hardware".to_string()),
                sector: Some("Technology".to_string()),
                ind_code: None,
                group_ranks: vec![],
                group_rs: vec![],
                number_of_stocks_in_group: Some(25),
            }),
            ..empty_item()
        };

        let records = flatten_market_data(&["AAPL"], &[item]);

        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.symbol, "AAPL");
        assert_eq!(r.company_name.as_deref(), Some("Apple Inc"));
        assert_eq!(r.instrument_type.as_deref(), Some("COMMON_STOCK"));
        assert_eq!(r.ipo_date.as_deref(), Some("1980-12-12"));
        assert_eq!(r.comp_rating, Some(95));
        assert_eq!(r.rs_rating, Some(92));
        assert_eq!(r.eps_rating, Some(88));
        assert_eq!(r.smr_rating.as_deref(), Some("A"));
        assert_eq!(r.ad_rating.as_deref(), Some("B+"));
        assert_eq!(r.industry_name.as_deref(), Some("Computers-Hardware"));
        assert_eq!(r.industry_sector.as_deref(), Some("Technology"));
        assert_eq!(r.industry_stocks_in_group, Some(25));
        assert!(r.decode_error.is_none());
    }

    #[test]
    fn flatten_partial_data() {
        let item = MdMarketDataItem {
            symbology: Some(MdSymbology {
                company: Some(MdCompany {
                    company_name: Some("Test Corp".to_string()),
                    address: None,
                    address2: None,
                    phone: None,
                    business_description: None,
                    url: None,
                    city: None,
                    country: None,
                    state_province: None,
                }),
                instrument: None,
            }),
            ..empty_item()
        };

        let records = flatten_market_data(&["TST"], &[item]);

        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.symbol, "TST");
        assert_eq!(r.company_name.as_deref(), Some("Test Corp"));
        assert!(r.instrument_type.is_none());
        assert!(r.ipo_date.is_none());
        assert!(r.comp_rating.is_none());
        assert!(r.rs_rating.is_none());
        assert!(r.eps_rating.is_none());
        assert!(r.smr_rating.is_none());
        assert!(r.ad_rating.is_none());
        assert!(r.market_cap.is_none());
        assert!(r.industry_name.is_none());
        assert!(r.funds_pct_float_held.is_none());
        assert!(r.eps_due_date.is_none());
        assert!(r.debt_pct.is_none());
        assert!(r.decode_error.is_none());
    }

    #[test]
    fn flatten_includes_per_symbol_decode_error() {
        let item = MdMarketDataItem {
            decode_error: Some("invalid type: map, expected a string".to_string()),
            ..empty_item()
        };

        let records = flatten_market_data(&["APLD"], &[item]);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].symbol, "APLD");
        assert_eq!(
            records[0].decode_error.as_deref(),
            Some("invalid type: map, expected a string")
        );
    }

    #[test]
    fn all_rows_failed_to_decode_requires_non_empty_all_error_rows() {
        assert!(!all_rows_failed_to_decode(&[]));

        let good_item = MdMarketDataItem {
            symbology: Some(MdSymbology {
                company: Some(MdCompany {
                    company_name: Some("Apple Inc".to_string()),
                    address: None,
                    address2: None,
                    phone: None,
                    business_description: None,
                    url: None,
                    city: None,
                    country: None,
                    state_province: None,
                }),
                instrument: None,
            }),
            ..empty_item()
        };
        let bad_item = MdMarketDataItem {
            decode_error: Some("invalid length 1, expected struct MdInstrument".to_string()),
            ..empty_item()
        };

        let mixed_records = flatten_market_data(&["AAPL", "NOW"], &[good_item, bad_item.clone()]);
        assert!(!all_rows_failed_to_decode(&mixed_records));

        let failed_records = flatten_market_data(&["NOW"], &[bad_item]);
        assert!(all_rows_failed_to_decode(&failed_records));
    }

    #[test]
    fn flatten_includes_nested_statistics_and_fundamentals() {
        let item = MdMarketDataItem {
            pricing_statistics: Some(MdPricingStatistics {
                end_of_day_statistics: Some(MdEndOfDayStatistics {
                    historical_price_statistics: vec![],
                    pricing_start_date: None,
                    pricing_end_date: None,
                    volume_moving_averages: vec![],
                    avg_dollar_volume_50_day: Some(FormattedFloat {
                        value: Some(25_000_000.0),
                        formatted_value: Some("25.0M".to_string()),
                    }),
                    market_capitalization: Some(FormattedFloat {
                        value: Some(1_500_000_000.0),
                        formatted_value: Some("1.5B".to_string()),
                    }),
                    average_true_range_percent: vec![],
                    ant_events: vec![],
                    up_down_volume_ratio: Some(MdScaledFloat {
                        value: Some(1.4),
                        scaling_factor: None,
                        formatted_value: Some("1.4".to_string()),
                    }),
                    alpha: None,
                    beta: None,
                    short_interest: Some(MdShortInterest {
                        days_to_cover: Some(FormattedFloat {
                            value: Some(2.5),
                            formatted_value: Some("2.5".to_string()),
                        }),
                        days_to_cover_percent_change: None,
                        percent_of_float: Some(MdScaledFloat {
                            value: Some(7.5),
                            scaling_factor: None,
                            formatted_value: Some("7.5%".to_string()),
                        }),
                        volume: None,
                    }),
                    blue_dot_daily_events: vec![],
                    blue_dot_weekly_events: vec![],
                }),
                intraday_statistics: None,
            }),
            financials: Some(MdFinancials {
                eps_due_date: Some(MdFormattedString {
                    value: Some("2026-01-30".to_string()),
                    formatted_value: Some("Jan 30, 2026".to_string()),
                }),
                eps_due_date_status: Some("CONFIRMED".to_string()),
                eps_last_reported_date: None,
                consensus_financials: None,
                cash_flow_per_share_last_year: None,
                profit_margin_values: vec![],
                estimates: None,
            }),
            ownership: Some(MdOwnership {
                funds_float_percent_held: Some(MdScaledFloat {
                    value: Some(62.5),
                    scaling_factor: None,
                    formatted_value: Some("62.5%".to_string()),
                }),
            }),
            fundamentals: Some(MdFundamentals {
                research_and_development_percent_last_qtr: Some(MdScaledFloat {
                    value: Some(4.2),
                    scaling_factor: None,
                    formatted_value: Some("4.2%".to_string()),
                }),
                new_ceo_date: None,
                debt_percent: Some(FormattedFloat {
                    value: Some(12.3),
                    formatted_value: Some("12.3%".to_string()),
                }),
            }),
            ..empty_item()
        };

        let records = flatten_market_data(&["APLD", "EXTRA"], &[item]);

        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.symbol, "APLD");
        assert_eq!(r.market_cap.as_deref(), Some("1.5B"));
        assert_eq!(r.avg_dollar_volume_50d.as_deref(), Some("25.0M"));
        assert_eq!(r.up_down_volume_ratio.as_deref(), Some("1.4"));
        assert_eq!(r.short_interest_pct_float.as_deref(), Some("7.5%"));
        assert_eq!(r.short_interest_days_to_cover.as_deref(), Some("2.5"));
        assert_eq!(r.funds_pct_float_held.as_deref(), Some("62.5%"));
        assert_eq!(r.eps_due_date.as_deref(), Some("Jan 30, 2026"));
        assert_eq!(r.eps_due_date_status.as_deref(), Some("CONFIRMED"));
        assert_eq!(r.debt_pct.as_deref(), Some("12.3%"));
        assert_eq!(r.rd_pct_last_qtr.as_deref(), Some("4.2%"));
    }

    #[test]
    fn flatten_empty_vec() {
        let records = flatten_market_data(&[], &[]);
        assert!(records.is_empty());
    }
}
