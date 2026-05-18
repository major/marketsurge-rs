//! RS rating data command.

use serde::Serialize;
use tracing::instrument;

use crate::cli::SymbolsArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

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
pub async fn handle(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .rs_rating_ri_panel(&symbol_refs, None)
                .await
                .map_err(handle_api_error)?;

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
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

            Ok(records)
        },
    )
    .await
}
