//! Chart OHLCV data command for daily and weekly price history.

use chrono::Utc;
use serde::Serialize;
use tracing::instrument;

use crate::cli::ChartArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

/// Flat output record for a single OHLCV data point.
///
/// Each row represents one period (day or week) for a symbol.
#[derive(Debug, Clone, Serialize)]
pub struct ChartRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Time series period (e.g. "ONE_DAY", "ONE_WEEK").
    pub period: String,
    /// Period start timestamp (ISO 8601).
    pub date: String,
    /// Opening price.
    pub open: Option<f64>,
    /// Period high price.
    pub high: Option<f64>,
    /// Period low price.
    pub low: Option<f64>,
    /// Closing price.
    pub close: Option<f64>,
    /// Trading volume.
    pub volume: Option<f64>,
}

/// ISO 8601 format with milliseconds for the MarketSurge API.
const DATE_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Handles the chart command.
#[instrument(skip_all)]
pub async fn handle(args: &ChartArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols.symbols,
        json_table,
        |client, symbol_refs| async move {
            let now = Utc::now();
            let end = now.format(DATE_FMT).to_string();

            let response = if args.weekly {
                let start = (now - chrono::Duration::weeks(156))
                    .format(DATE_FMT)
                    .to_string();
                client
                    .chart_market_data_weekly(&symbol_refs, "CHARTING", &start, &end)
                    .await
                    .map_err(handle_api_error)?
            } else {
                let start = (now - chrono::Duration::days(365))
                    .format(DATE_FMT)
                    .to_string();
                client
                    .chart_market_data(
                        &symbol_refs,
                        "CHARTING",
                        &start,
                        &end,
                        "ONE_DAY",
                        true,
                        "NYSE",
                    )
                    .await
                    .map_err(handle_api_error)?
            };

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
                let pricing = match &item.pricing {
                    Some(p) => p,
                    None => continue,
                };
                let ts = match &pricing.time_series {
                    Some(ts) => ts,
                    None => continue,
                };

                for dp in &ts.data_points {
                    records.push(ChartRecord {
                        symbol: symbol.to_string(),
                        period: ts.period.clone(),
                        date: dp.start_date_time.clone(),
                        open: dp.open.as_ref().and_then(|v| v.value),
                        high: dp.high.as_ref().and_then(|v| v.value),
                        low: dp.low.as_ref().and_then(|v| v.value),
                        close: dp.last.as_ref().and_then(|v| v.value),
                        volume: dp.volume.as_ref().and_then(|v| v.value),
                    });
                }
            }

            Ok(records)
        },
    )
    .await
}
