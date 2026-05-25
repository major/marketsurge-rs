//! RS rating data command.

use crate::ratings::RsRatingRiPanelResponse;
use serde::Serialize;
use tracing::instrument;

use crate::cli::SymbolsArgs;
use crate::cli::common::command::{api_call, run_command, zip_symbols};

/// Flat output record for a single RS rating snapshot.
///
/// Each row represents one period/offset combination for a symbol.
/// The `rs_line_new_high` field is repeated on every row for the same
/// symbol since it is not period-specific.
#[derive(Debug, Clone, Serialize)]
pub struct RatingsRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Rating period (e.g. "DAILY").
    pub period: Option<String>,
    /// Period offset (e.g. "CURRENT", "P1W_AGO").
    pub period_offset: Option<String>,
    /// Letter grade (e.g. "A", "B").
    pub letter_value: Option<String>,
    /// Numeric RS rating value (1-99).
    pub value: Option<i64>,
    /// Whether the RS line is at a new high.
    pub rs_line_new_high: Option<bool>,
}

/// Handles the ratings command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &SymbolsArgs, fields: &[String]) -> i32 {
    run_command(&args.symbols, fields, |client, symbol_refs| async move {
        let response = api_call(client.rs_rating_ri_panel(&symbol_refs, None)).await?;

        Ok(flatten_ratings(&symbol_refs, response))
    })
    .await
}

fn flatten_ratings(symbol_refs: &[&str], response: RsRatingRiPanelResponse) -> Vec<RatingsRecord> {
    let mut records = Vec::new();

    for (symbol, item) in zip_symbols(symbol_refs, &response.market_data) {
        let rs_line_new_high = item
            .pricing_statistics
            .as_ref()
            .and_then(|p| p.intraday_statistics.as_ref())
            .and_then(|i| i.rs_line_new_high);

        let snapshots = item
            .ratings
            .as_ref()
            .map(|r| r.rs_rating.as_slice())
            .unwrap_or_default();

        if snapshots.is_empty() {
            records.push(RatingsRecord {
                symbol: symbol.to_string(),
                period: None,
                period_offset: None,
                letter_value: None,
                value: None,
                rs_line_new_high,
            });
        } else {
            for snap in snapshots {
                records.push(RatingsRecord {
                    symbol: symbol.to_string(),
                    period: snap.period.clone(),
                    period_offset: snap.period_offset.clone(),
                    letter_value: snap.letter_value.clone(),
                    value: snap.value,
                    rs_line_new_high,
                });
            }
        }
    }

    records
}

#[cfg(test)]
mod tests {
    use super::flatten_ratings;
    use crate::ratings::{
        RsRatingIntradayStatistics, RsRatingPricingStatistics, RsRatingRatings,
        RsRatingRiPanelItem, RsRatingRiPanelResponse, RsRatingSnapshot,
    };

    fn snapshot(
        letter_value: Option<&str>,
        period: Option<&str>,
        period_offset: Option<&str>,
        value: Option<i64>,
    ) -> RsRatingSnapshot {
        RsRatingSnapshot {
            letter_value: letter_value.map(str::to_string),
            period: period.map(str::to_string),
            period_offset: period_offset.map(str::to_string),
            value,
        }
    }

    fn item(
        rs_line_new_high: Option<bool>,
        snapshots: Vec<RsRatingSnapshot>,
    ) -> RsRatingRiPanelItem {
        RsRatingRiPanelItem {
            id: None,
            origin_request: None,
            ratings: Some(RsRatingRatings {
                rs_rating: snapshots,
            }),
            pricing_statistics: Some(RsRatingPricingStatistics {
                intraday_statistics: Some(RsRatingIntradayStatistics { rs_line_new_high }),
            }),
        }
    }

    fn item_without_snapshots(rs_line_new_high: Option<bool>) -> RsRatingRiPanelItem {
        RsRatingRiPanelItem {
            id: None,
            origin_request: None,
            ratings: Some(RsRatingRatings { rs_rating: vec![] }),
            pricing_statistics: Some(RsRatingPricingStatistics {
                intraday_statistics: Some(RsRatingIntradayStatistics { rs_line_new_high }),
            }),
        }
    }

    #[test]
    fn flatten_ratings_expands_snapshots_and_propagates_flag() {
        let response = RsRatingRiPanelResponse {
            market_data: vec![item(
                Some(true),
                vec![
                    snapshot(Some("A"), Some("DAILY"), Some("CURRENT"), Some(92)),
                    snapshot(Some("B"), Some("WEEKLY"), Some("P1W_AGO"), Some(85)),
                ],
            )],
        };

        let records = flatten_ratings(&["AAPL"], response);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].period.as_deref(), Some("DAILY"));
        assert_eq!(records[0].period_offset.as_deref(), Some("CURRENT"));
        assert_eq!(records[0].letter_value.as_deref(), Some("A"));
        assert_eq!(records[0].value, Some(92));
        assert_eq!(records[0].rs_line_new_high, Some(true));
        assert_eq!(records[1].symbol, "AAPL");
        assert_eq!(records[1].period.as_deref(), Some("WEEKLY"));
        assert_eq!(records[1].period_offset.as_deref(), Some("P1W_AGO"));
        assert_eq!(records[1].letter_value.as_deref(), Some("B"));
        assert_eq!(records[1].value, Some(85));
        assert_eq!(records[1].rs_line_new_high, Some(true));
    }

    #[test]
    fn flatten_ratings_falls_back_to_single_record_for_empty_snapshots() {
        let response = RsRatingRiPanelResponse {
            market_data: vec![item_without_snapshots(Some(true))],
        };

        let records = flatten_ratings(&["AAPL"], response);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].period, None);
        assert_eq!(records[0].period_offset, None);
        assert_eq!(records[0].letter_value, None);
        assert_eq!(records[0].value, None);
        assert_eq!(records[0].rs_line_new_high, Some(true));
    }

    #[test]
    fn flatten_ratings_propagates_false_flag_to_all_records() {
        let response = RsRatingRiPanelResponse {
            market_data: vec![item(
                Some(false),
                vec![
                    snapshot(Some("A"), Some("DAILY"), Some("CURRENT"), Some(92)),
                    snapshot(Some("B"), Some("WEEKLY"), Some("P1W_AGO"), Some(85)),
                ],
            )],
        };

        let records = flatten_ratings(&["AAPL"], response);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].rs_line_new_high, Some(false));
        assert_eq!(records[1].rs_line_new_high, Some(false));
    }
}
