//! Chart OHLCV data command for daily and weekly price history.

use chrono::Utc;
use serde::Serialize;
use tracing::instrument;

use crate::cli::ChartArgs;
use crate::common::command::{api_call, run_command, zip_symbols};
use marketsurge_client::chart::ChartMarketDataResponse;

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

/// Flattens chart market data into OHLCV output records.
pub(crate) fn flatten_chart_data(
    symbols: &[&str],
    response: ChartMarketDataResponse,
) -> Vec<ChartRecord> {
    let mut records = Vec::new();

    for (symbol, item) in zip_symbols(symbols, &response.market_data) {
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

    records
}

/// Handles the chart command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &ChartArgs, fields: &[String]) -> i32 {
    run_command(
        &args.symbols.symbols,
        fields,
        |client, symbol_refs| async move {
            let now = Utc::now();
            let end = now.format(DATE_FMT).to_string();

            let response = if args.weekly {
                let start = (now - chrono::Duration::weeks(156))
                    .format(DATE_FMT)
                    .to_string();
                api_call(client.chart_market_data_weekly(&symbol_refs, "CHARTING", &start, &end))
                    .await?
            } else {
                let start = (now - chrono::Duration::days(365))
                    .format(DATE_FMT)
                    .to_string();
                api_call(client.chart_market_data(
                    &symbol_refs,
                    "CHARTING",
                    &start,
                    &end,
                    "ONE_DAY",
                    true,
                    "NYSE",
                ))
                .await?
            };

            Ok(flatten_chart_data(&symbol_refs, response))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::flatten_chart_data;
    use marketsurge_client::chart::{
        ChartDataPoint, ChartMarketDataItem, ChartMarketDataResponse, ChartPricing, ChartTimeSeries,
    };

    fn chart_value(value: f64) -> marketsurge_client::types::FloatValue {
        marketsurge_client::types::FloatValue { value: Some(value) }
    }

    fn data_point(
        start_date_time: &str,
        open: Option<f64>,
        high: Option<f64>,
        low: Option<f64>,
        close: Option<f64>,
        volume: Option<f64>,
    ) -> ChartDataPoint {
        ChartDataPoint {
            start_date_time: start_date_time.to_string(),
            end_date_time: String::new(),
            volume: volume.map(chart_value),
            last: close.map(chart_value),
            low: low.map(chart_value),
            high: high.map(chart_value),
            open: open.map(chart_value),
        }
    }

    fn chart_item(period: &str, data_points: Vec<ChartDataPoint>) -> ChartMarketDataItem {
        ChartMarketDataItem {
            id: String::new(),
            origin_request: None,
            pricing: Some(ChartPricing {
                time_series: Some(ChartTimeSeries {
                    period: period.to_string(),
                    data_points,
                }),
                quote: None,
                premarket_quote: None,
                postmarket_quote: None,
                current_market_state: None,
            }),
        }
    }

    fn response(items: Vec<ChartMarketDataItem>) -> ChartMarketDataResponse {
        ChartMarketDataResponse {
            market_data: items,
            exchange_data: None,
        }
    }

    #[test]
    fn flatten_chart_data_happy_path() {
        let symbols = ["AAPL"];
        let response = response(vec![chart_item(
            "ONE_DAY",
            vec![
                data_point(
                    "2025-05-01T00:00:00.000Z",
                    Some(10.0),
                    Some(12.0),
                    Some(9.0),
                    Some(11.0),
                    Some(1000.0),
                ),
                data_point(
                    "2025-05-02T00:00:00.000Z",
                    Some(11.0),
                    Some(13.0),
                    Some(10.0),
                    Some(12.0),
                    Some(1500.0),
                ),
            ],
        )]);

        let records = flatten_chart_data(&symbols, response);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].period, "ONE_DAY");
        assert_eq!(records[0].date, "2025-05-01T00:00:00.000Z");
        assert_eq!(records[0].open, Some(10.0));
        assert_eq!(records[0].high, Some(12.0));
        assert_eq!(records[0].low, Some(9.0));
        assert_eq!(records[0].close, Some(11.0));
        assert_eq!(records[0].volume, Some(1000.0));

        assert_eq!(records[1].symbol, "AAPL");
        assert_eq!(records[1].period, "ONE_DAY");
        assert_eq!(records[1].date, "2025-05-02T00:00:00.000Z");
        assert_eq!(records[1].open, Some(11.0));
        assert_eq!(records[1].high, Some(13.0));
        assert_eq!(records[1].low, Some(10.0));
        assert_eq!(records[1].close, Some(12.0));
        assert_eq!(records[1].volume, Some(1500.0));
    }

    #[test]
    fn flatten_chart_data_empty_market_data() {
        let symbols = ["AAPL"];

        let records = flatten_chart_data(&symbols, response(vec![]));

        assert!(records.is_empty());
    }

    #[test]
    fn flatten_chart_data_empty_data_points() {
        let symbols = ["AAPL"];
        let response = response(vec![chart_item("ONE_DAY", vec![])]);

        let records = flatten_chart_data(&symbols, response);

        assert!(records.is_empty());
    }
}
