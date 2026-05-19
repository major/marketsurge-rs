//! Fundamental financial data command.

use serde::Serialize;
use tracing::instrument;

use marketsurge_client::fundamentals::FundamentalsItem;

use crate::cli::SymbolsArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

/// Flat output record for a single fundamentals period.
///
/// Each row represents one reported or estimated period for a metric
/// (EPS or sales). Use the `metric` field to filter by type.
#[derive(Debug, Clone, Serialize)]
pub struct FundamentalsRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Company name.
    pub company_name: Option<String>,
    /// Metric type: `reported_eps`, `reported_sales`, `eps_estimate`,
    /// or `sales_estimate`.
    pub metric: String,
    /// Period offset (e.g. "CURRENT", "P1Q_AGO", "P1Q_FUTURE").
    pub period_offset: Option<String>,
    /// Period end date (reported) or period identifier (estimates).
    pub period: Option<String>,
    /// Formatted display value (e.g. "$1.65", "$95.2B").
    pub value: Option<String>,
    /// Year-over-year percent change (formatted).
    pub pct_change_yoy: Option<String>,
    /// Estimate revision direction ("UP", "DOWN"), EPS estimates only.
    pub revision_direction: Option<String>,
}

/// Flattens nested fundamentals data into a flat list of records.
///
/// Each [`FundamentalsItem`] may contain up to four metric branches:
/// reported EPS, reported sales, EPS estimates, and sales estimates.
/// Items with `financials: None` are skipped.
fn flatten_fundamentals(
    symbols: &[&str],
    market_data: &[FundamentalsItem],
) -> Vec<FundamentalsRecord> {
    let mut records = Vec::new();

    for (symbol, item) in zip_symbols(symbols, market_data) {
        let company_name = item
            .symbology
            .as_ref()
            .and_then(|s| s.company.as_ref())
            .and_then(|c| c.company_name.clone());

        let financials = match &item.financials {
            Some(f) => f,
            None => continue,
        };

        // Reported EPS
        if let Some(eps) = financials
            .consensus_financials
            .as_ref()
            .and_then(|c| c.eps.as_ref())
        {
            for period in &eps.reported_earnings {
                records.push(FundamentalsRecord {
                    symbol: symbol.to_string(),
                    company_name: company_name.clone(),
                    metric: "reported_eps".to_string(),
                    period_offset: period.period_offset.clone(),
                    period: period
                        .period_end_date
                        .as_ref()
                        .and_then(|d| d.value.clone()),
                    value: period
                        .value
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    pct_change_yoy: period
                        .percent_change_yoy
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    revision_direction: None,
                });
            }
        }

        // Reported sales
        if let Some(sales) = financials
            .consensus_financials
            .as_ref()
            .and_then(|c| c.sales.as_ref())
        {
            for period in &sales.reported_sales {
                records.push(FundamentalsRecord {
                    symbol: symbol.to_string(),
                    company_name: company_name.clone(),
                    metric: "reported_sales".to_string(),
                    period_offset: period.period_offset.clone(),
                    period: period
                        .period_end_date
                        .as_ref()
                        .and_then(|d| d.value.clone()),
                    value: period
                        .value
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    pct_change_yoy: period
                        .percent_change_yoy
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    revision_direction: None,
                });
            }
        }

        // EPS estimates
        if let Some(estimates) = &financials.estimates {
            for est in &estimates.eps_estimates {
                records.push(FundamentalsRecord {
                    symbol: symbol.to_string(),
                    company_name: company_name.clone(),
                    metric: "eps_estimate".to_string(),
                    period_offset: est.period_offset.clone(),
                    period: est.period.clone(),
                    value: est.value.as_ref().and_then(|v| v.formatted_value.clone()),
                    pct_change_yoy: est
                        .percent_change_yoy
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    revision_direction: est.revision_direction.clone(),
                });
            }

            // Sales estimates
            for est in &estimates.sales_estimates {
                records.push(FundamentalsRecord {
                    symbol: symbol.to_string(),
                    company_name: company_name.clone(),
                    metric: "sales_estimate".to_string(),
                    period_offset: est.period_offset.clone(),
                    period: est.period.clone(),
                    value: est.value.as_ref().and_then(|v| v.formatted_value.clone()),
                    pct_change_yoy: est
                        .percent_change_yoy
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                    revision_direction: None,
                });
            }
        }
    }

    records
}

/// Handles the fundamentals command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .fundamentals(
                    &symbol_refs,
                    "CHARTING",
                    "P7Y_AGO",
                    "P2Y_FUTURE",
                    "P7Y_AGO",
                    "P2Y_FUTURE",
                )
                .await
                .map_err(handle_api_error)?;

            Ok(flatten_fundamentals(&symbol_refs, &response.market_data))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use marketsurge_client::fundamentals::{
        FundamentalsCompany, FundamentalsConsensus, FundamentalsConsensusEps,
        FundamentalsConsensusSales, FundamentalsEstimate, FundamentalsEstimates,
        FundamentalsFinancials, FundamentalsItem, FundamentalsReportedPeriod,
        FundamentalsSymbology,
    };
    use marketsurge_client::types::{DateValue, FormattedFloat};

    use super::*;

    fn make_formatted(val: f64, display: &str) -> Option<FormattedFloat> {
        Some(FormattedFloat {
            value: Some(val),
            formatted_value: Some(display.to_string()),
        })
    }

    fn make_item_with_financials(financials: FundamentalsFinancials) -> FundamentalsItem {
        FundamentalsItem {
            id: Some("AAPL".to_string()),
            financials: Some(financials),
            symbology: Some(FundamentalsSymbology {
                company: Some(FundamentalsCompany {
                    company_name: Some("Apple Inc.".to_string()),
                }),
                instrument: None,
            }),
        }
    }

    #[test]
    fn reported_eps_branch() {
        let item = make_item_with_financials(FundamentalsFinancials {
            consensus_financials: Some(FundamentalsConsensus {
                eps: Some(FundamentalsConsensusEps {
                    reported_earnings: vec![FundamentalsReportedPeriod {
                        value: make_formatted(1.65, "$1.65"),
                        percent_change_yoy: make_formatted(10.0, "10%"),
                        period_offset: Some("CURRENT".to_string()),
                        period_end_date: Some(DateValue {
                            value: Some("2026-03-31".to_string()),
                        }),
                    }],
                }),
                sales: None,
            }),
            estimates: None,
        });

        let records = flatten_fundamentals(&["AAPL"], &[item]);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].metric, "reported_eps");
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].company_name.as_deref(), Some("Apple Inc."));
        assert_eq!(records[0].value.as_deref(), Some("$1.65"));
        assert_eq!(records[0].pct_change_yoy.as_deref(), Some("10%"));
        assert_eq!(records[0].period_offset.as_deref(), Some("CURRENT"));
        assert_eq!(records[0].period.as_deref(), Some("2026-03-31"));
        assert!(records[0].revision_direction.is_none());
    }

    #[test]
    fn reported_sales_branch() {
        let item = make_item_with_financials(FundamentalsFinancials {
            consensus_financials: Some(FundamentalsConsensus {
                eps: None,
                sales: Some(FundamentalsConsensusSales {
                    reported_sales: vec![FundamentalsReportedPeriod {
                        value: make_formatted(95.2, "$95.2B"),
                        percent_change_yoy: make_formatted(5.0, "5%"),
                        period_offset: Some("P1Q_AGO".to_string()),
                        period_end_date: Some(DateValue {
                            value: Some("2025-12-31".to_string()),
                        }),
                    }],
                }),
            }),
            estimates: None,
        });

        let records = flatten_fundamentals(&["AAPL"], &[item]);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].metric, "reported_sales");
        assert_eq!(records[0].value.as_deref(), Some("$95.2B"));
        assert_eq!(records[0].period.as_deref(), Some("2025-12-31"));
        assert!(records[0].revision_direction.is_none());
    }

    #[test]
    fn eps_estimate_branch() {
        let item = make_item_with_financials(FundamentalsFinancials {
            consensus_financials: None,
            estimates: Some(FundamentalsEstimates {
                eps_estimates: vec![FundamentalsEstimate {
                    value: make_formatted(1.72, "$1.72"),
                    percent_change_yoy: make_formatted(4.2, "4.2%"),
                    period_offset: Some("P1Q_FUTURE".to_string()),
                    period: Some("P1Q".to_string()),
                    revision_direction: Some("UP".to_string()),
                }],
                sales_estimates: vec![],
            }),
        });

        let records = flatten_fundamentals(&["AAPL"], &[item]);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].metric, "eps_estimate");
        assert_eq!(records[0].value.as_deref(), Some("$1.72"));
        assert_eq!(records[0].period.as_deref(), Some("P1Q"));
        assert_eq!(records[0].revision_direction.as_deref(), Some("UP"));
    }

    #[test]
    fn sales_estimate_branch() {
        let item = make_item_with_financials(FundamentalsFinancials {
            consensus_financials: None,
            estimates: Some(FundamentalsEstimates {
                eps_estimates: vec![],
                sales_estimates: vec![FundamentalsEstimate {
                    value: make_formatted(100.5, "$100.5B"),
                    percent_change_yoy: make_formatted(8.0, "8%"),
                    period_offset: Some("P1Q_FUTURE".to_string()),
                    period: Some("P1Q".to_string()),
                    revision_direction: Some("DOWN".to_string()),
                }],
            }),
        });

        let records = flatten_fundamentals(&["AAPL"], &[item]);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].metric, "sales_estimate");
        assert_eq!(records[0].value.as_deref(), Some("$100.5B"));
        assert_eq!(records[0].period.as_deref(), Some("P1Q"));
        // Sales estimates never carry revision_direction
        assert!(records[0].revision_direction.is_none());
    }

    #[test]
    fn none_financials_produces_no_records() {
        let item = FundamentalsItem {
            id: Some("AAPL".to_string()),
            financials: None,
            symbology: None,
        };

        let records = flatten_fundamentals(&["AAPL"], &[item]);

        assert!(records.is_empty());
    }
}
