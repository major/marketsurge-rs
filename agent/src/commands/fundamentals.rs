//! Fundamental financial data command.

use serde::Serialize;
use tracing::instrument;

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

/// Handles the fundamentals command.
#[instrument(skip_all)]
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

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
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

            Ok(records)
        },
    )
    .await
}
