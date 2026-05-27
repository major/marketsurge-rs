//! Chart OHLCV data command for daily and weekly price history.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use tracing::instrument;

use crate::chart::ChartMarketDataResponse;
use crate::cli::ChartArgs;
use crate::cli::common::command::{api_call, run_command, zip_symbols};

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

#[derive(Debug, Clone, Copy)]
struct ChartDateRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl ChartDateRange {
    fn formatted_start(self) -> String {
        self.start.format(DATE_FMT).to_string()
    }

    fn formatted_end(self) -> String {
        self.end.format(DATE_FMT).to_string()
    }
}

fn start_of_day_utc(date: NaiveDate) -> DateTime<Utc> {
    date.and_time(NaiveTime::MIN).and_utc()
}

fn chart_date_range(args: &ChartArgs, now: DateTime<Utc>) -> ChartDateRange {
    let start = if let Some(start_date) = args.start_date {
        start_of_day_utc(start_date)
    } else if let Some(days) = args.days {
        range_start_for_bar_count(now, args.weekly, days)
    } else if args.weekly {
        now - Duration::weeks(156)
    } else {
        now - Duration::days(365)
    };

    ChartDateRange { start, end: now }
}

fn range_start_for_bar_count(now: DateTime<Utc>, weekly: bool, days: u16) -> DateTime<Utc> {
    if weekly {
        now - Duration::weeks(i64::from(days) + 2)
    } else {
        now - Duration::days(i64::from(days) * 2 + 7)
    }
}

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

fn keep_last_bars_per_symbol(records: Vec<ChartRecord>, count: Option<u16>) -> Vec<ChartRecord> {
    let Some(count) = count else {
        return records;
    };
    let count = usize::from(count);

    let mut kept = Vec::new();
    let mut current_symbol = None::<String>;
    let mut current_records = Vec::new();

    for record in records {
        if current_symbol.as_deref() != Some(record.symbol.as_str()) {
            keep_last_records(&mut kept, &mut current_records, count);
            current_symbol = Some(record.symbol.clone());
        }
        current_records.push(record);
    }

    keep_last_records(&mut kept, &mut current_records, count);
    kept
}

fn keep_last_records(
    kept: &mut Vec<ChartRecord>,
    current_records: &mut Vec<ChartRecord>,
    count: usize,
) {
    if current_records.len() > count {
        current_records.drain(0..current_records.len() - count);
    }
    kept.append(current_records);
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
            let date_range = chart_date_range(args, now);
            let start = date_range.formatted_start();
            let end = date_range.formatted_end();

            let response = if args.weekly {
                api_call(client.chart_market_data_weekly(&symbol_refs, "CHARTING", &start, &end))
                    .await?
            } else {
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

            Ok(keep_last_bars_per_symbol(
                flatten_chart_data(&symbol_refs, response),
                args.days,
            ))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{chart_date_range, flatten_chart_data, keep_last_bars_per_symbol};
    use crate::chart::{
        ChartDataPoint, ChartMarketDataItem, ChartMarketDataResponse, ChartPricing, ChartTimeSeries,
    };
    use crate::cli::{ChartArgs, SymbolsArgs};

    fn chart_args(weekly: bool, days: Option<u16>, start_date: Option<&str>) -> ChartArgs {
        ChartArgs {
            weekly,
            days,
            start_date: start_date.map(|date| date.parse().expect("valid test date")),
            symbols: SymbolsArgs {
                symbols: vec!["AAPL".to_string()],
            },
        }
    }

    fn chart_value(value: f64) -> crate::types::FloatValue {
        crate::types::FloatValue { value: Some(value) }
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
    fn chart_date_range_defaults_to_one_year_for_daily_bars() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let range = chart_date_range(&chart_args(false, None, None), now);

        assert_eq!(range.formatted_start(), "2025-05-27T12:00:00.000Z");
        assert_eq!(range.formatted_end(), "2026-05-27T12:00:00.000Z");
    }

    #[test]
    fn chart_date_range_defaults_to_three_years_for_weekly_bars() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let range = chart_date_range(&chart_args(true, None, None), now);

        assert_eq!(range.formatted_start(), "2023-05-31T12:00:00.000Z");
        assert_eq!(range.formatted_end(), "2026-05-27T12:00:00.000Z");
    }

    #[test]
    fn chart_date_range_uses_start_date_at_midnight() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let range = chart_date_range(&chart_args(false, None, Some("2026-05-01")), now);

        assert_eq!(range.formatted_start(), "2026-05-01T00:00:00.000Z");
        assert_eq!(range.formatted_end(), "2026-05-27T12:00:00.000Z");
    }

    #[test]
    fn chart_date_range_expands_daily_days_window_for_trading_days() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let range = chart_date_range(&chart_args(false, Some(10), None), now);

        assert_eq!(range.formatted_start(), "2026-04-30T12:00:00.000Z");
    }

    #[test]
    fn chart_date_range_expands_weekly_days_window_for_weekly_bars() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let range = chart_date_range(&chart_args(true, Some(20), None), now);

        assert_eq!(range.formatted_start(), "2025-12-24T12:00:00.000Z");
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

    #[test]
    fn keep_last_bars_per_symbol_trims_each_symbol_independently() {
        let symbols = ["AAPL", "MSFT"];
        let response = response(vec![
            chart_item(
                "ONE_DAY",
                vec![
                    data_point(
                        "2026-05-01T00:00:00.000Z",
                        None,
                        None,
                        None,
                        Some(1.0),
                        None,
                    ),
                    data_point(
                        "2026-05-02T00:00:00.000Z",
                        None,
                        None,
                        None,
                        Some(2.0),
                        None,
                    ),
                    data_point(
                        "2026-05-03T00:00:00.000Z",
                        None,
                        None,
                        None,
                        Some(3.0),
                        None,
                    ),
                ],
            ),
            chart_item(
                "ONE_DAY",
                vec![
                    data_point(
                        "2026-05-01T00:00:00.000Z",
                        None,
                        None,
                        None,
                        Some(4.0),
                        None,
                    ),
                    data_point(
                        "2026-05-02T00:00:00.000Z",
                        None,
                        None,
                        None,
                        Some(5.0),
                        None,
                    ),
                ],
            ),
        ]);
        let records = flatten_chart_data(&symbols, response);

        let records = keep_last_bars_per_symbol(records, Some(2));

        assert_eq!(records.len(), 4);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].date, "2026-05-02T00:00:00.000Z");
        assert_eq!(records[1].date, "2026-05-03T00:00:00.000Z");
        assert_eq!(records[2].symbol, "MSFT");
        assert_eq!(records[2].date, "2026-05-01T00:00:00.000Z");
        assert_eq!(records[3].date, "2026-05-02T00:00:00.000Z");
    }

    #[test]
    fn keep_last_bars_per_symbol_keeps_all_records_without_count() {
        let symbols = ["AAPL"];
        let records = flatten_chart_data(
            &symbols,
            response(vec![chart_item(
                "ONE_DAY",
                vec![
                    data_point("2026-05-01T00:00:00.000Z", None, None, None, None, None),
                    data_point("2026-05-02T00:00:00.000Z", None, None, None, None, None),
                ],
            )]),
        );

        let records = keep_last_bars_per_symbol(records, None);

        assert_eq!(records.len(), 2);
    }
}
