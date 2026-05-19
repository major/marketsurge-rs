//! Fund ownership data commands.

use clap::Subcommand;
use marketsurge_client::fundamentals::FundamentalsItem;
use marketsurge_client::ownership::OwnershipItem;
use marketsurge_client::screen::{ResponseValue, ScreenerParameter};
use serde::Serialize;
use tracing::instrument;

use crate::cli::{OwnershipArgs, SymbolsArgs};
use crate::common::command::{api_call, run_command, zip_symbols};

/// Screen name for the fund ownership detail query.
const FUND_OWNERSHIP_SCREEN: &str = "MarketSurge.RelatedInformation.MUTIFundOwnership";

/// Ownership subcommands.
#[derive(Debug, Subcommand)]
pub enum OwnershipCommand {
    /// Fetch quarterly fund ownership summary rows for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent ownership summary AAPL MSFT")]
    Summary(SymbolsArgs),
    /// Fetch individual fund holders and share history for symbols.
    #[command(after_help = "Examples:\n  marketsurge-agent ownership funds AAPL MSFT")]
    Funds(SymbolsArgs),
}

/// Flat output record for a single quarter's fund ownership data.
#[derive(Debug, Clone, Serialize)]
pub struct OwnershipSummaryRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Percentage of float held by funds.
    pub funds_float_pct_held: Option<String>,
    /// Quarter date.
    pub date: Option<String>,
    /// Number of funds holding the stock.
    pub num_funds_held: Option<String>,
}

/// Output record for a single fund's holdings in a queried stock.
#[derive(Debug, Clone, Serialize)]
pub struct FundOwnershipRecord {
    /// Stock ticker that was queried.
    pub queried_symbol: String,
    /// Fund ticker symbol.
    pub fund_symbol: Option<String>,
    /// Fund name.
    pub fund_name: Option<String>,
    /// Holdings as percent of fund assets held.
    pub holdings_pct: Option<String>,
    /// Number of shares held one quarter ago.
    pub shares_held_1q_ago: Option<String>,
    /// Date for one quarter ago holdings.
    pub date_1q_ago: Option<String>,
    /// Number of shares held two quarters ago.
    pub shares_held_2q_ago: Option<String>,
    /// Date for two quarters ago holdings.
    pub date_2q_ago: Option<String>,
    /// Number of shares held three quarters ago.
    pub shares_held_3q_ago: Option<String>,
    /// Date for three quarters ago holdings.
    pub date_3q_ago: Option<String>,
    /// Number of shares held four quarters ago.
    pub shares_held_4q_ago: Option<String>,
    /// Date for four quarters ago holdings.
    pub date_4q_ago: Option<String>,
}

/// Handles the ownership command group.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &OwnershipArgs, json_table: bool) -> i32 {
    match &args.command {
        OwnershipCommand::Summary(a) => execute_summary(a, json_table).await,
        OwnershipCommand::Funds(a) => execute_funds(a, json_table).await,
    }
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_summary(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = api_call(client.ownership(&symbol_refs)).await?;

            Ok(flatten_ownership_summary(
                &symbol_refs,
                &response.market_data,
            ))
        },
    )
    .await
}

#[instrument(skip_all)]
#[cfg(not(coverage))]
async fn execute_funds(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            // Resolve DJ_KEY for each symbol via the fundamentals API symbology.
            let fundamentals = api_call(client.fundamentals(
                &symbol_refs,
                "CHARTING",
                "P7Y_AGO",
                "P2Y_FUTURE",
                "P7Y_AGO",
                "P2Y_FUTURE",
            ))
            .await?;

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &fundamentals.market_data) {
                // Extract DJ_KEY from symbology and split into exchange + id.
                let dj_key = extract_dj_key(item);

                let (exchange, id) = match dj_key.and_then(|k| k.split_once('-')) {
                    Some(pair) => pair,
                    None => {
                        tracing::warn!(%symbol, "no DJ_KEY found, skipping fund lookup");
                        continue;
                    }
                };

                let parameters = vec![
                    ScreenerParameter {
                        name: "DowJonesExchange".to_string(),
                        value: exchange.to_string(),
                    },
                    ScreenerParameter {
                        name: "DowJonesId".to_string(),
                        value: id.to_string(),
                    },
                ];

                let response =
                    api_call(client.market_data_screen(FUND_OWNERSHIP_SCREEN, parameters)).await?;

                if let Some(result) = response.market_data_screen {
                    records.extend(flatten_fund_rows(symbol, &result.response_values));
                }
            }

            Ok(records)
        },
    )
    .await
}

/// Extracts a cell value from a screen response row by mdItem name.
///
/// Returns `None` if the column is missing or the value is empty.
fn cell_value(row: &[ResponseValue], name: &str) -> Option<String> {
    row.iter()
        .find(|cell| cell.md_item.as_ref().and_then(|m| m.name.as_deref()) == Some(name))
        .and_then(|cell| cell.value.clone())
        .filter(|v| !v.is_empty())
}

/// Flattens ownership response data into one record per symbol per quarter.
///
/// Symbols whose ownership is `None` are skipped. Symbols with ownership but
/// an empty `fund_ownership_summary` produce a single record with no date or
/// fund count.
fn flatten_ownership_summary(
    symbols: &[&str],
    market_data: &[OwnershipItem],
) -> Vec<OwnershipSummaryRecord> {
    let mut records = Vec::new();
    for (symbol, item) in zip_symbols(symbols, market_data) {
        let ownership = match &item.ownership {
            Some(o) => o,
            None => continue,
        };

        let pct_held = ownership
            .funds_float_percent_held
            .as_ref()
            .and_then(|v| v.formatted_value.clone());

        if ownership.fund_ownership_summary.is_empty() {
            records.push(OwnershipSummaryRecord {
                symbol: symbol.to_string(),
                funds_float_pct_held: pct_held,
                date: None,
                num_funds_held: None,
            });
        } else {
            for quarter in &ownership.fund_ownership_summary {
                records.push(OwnershipSummaryRecord {
                    symbol: symbol.to_string(),
                    funds_float_pct_held: pct_held.clone(),
                    date: quarter.date.as_ref().and_then(|d| d.value.clone()),
                    num_funds_held: quarter
                        .number_of_funds_held
                        .as_ref()
                        .and_then(|v| v.formatted_value.clone()),
                });
            }
        }
    }
    records
}

/// Extracts the DJ_KEY symbol value from a fundamentals item's symbology.
///
/// Navigates: symbology -> instrument -> symbols, then finds the entry with
/// `node_type == "DJ_KEY"` and returns its value.
fn extract_dj_key(item: &FundamentalsItem) -> Option<&str> {
    item.symbology
        .as_ref()
        .and_then(|s| s.instrument.as_ref())
        .map(|inst| &inst.symbols)
        .and_then(|symbols| {
            symbols
                .iter()
                .find(|s| s.node_type.as_deref() == Some("DJ_KEY"))
        })
        .and_then(|s| s.value.as_deref())
}

/// Converts screen response rows into flat fund ownership records.
///
/// Each row in `response_values` becomes one [`FundOwnershipRecord`] tagged
/// with the queried `symbol`. Missing or empty cells produce `None` fields.
fn flatten_fund_rows(
    symbol: &str,
    response_values: &[Vec<ResponseValue>],
) -> Vec<FundOwnershipRecord> {
    response_values
        .iter()
        .map(|row| FundOwnershipRecord {
            queried_symbol: symbol.to_string(),
            fund_symbol: cell_value(row, "Symbol"),
            fund_name: cell_value(row, "CompanyName"),
            holdings_pct: cell_value(row, "HoldingsPctFundAssetsHeld"),
            shares_held_1q_ago: cell_value(row, "NumberOfFunds1QAgo"),
            date_1q_ago: cell_value(row, "NumberOfFundsDate1QAgo"),
            shares_held_2q_ago: cell_value(row, "NumberOfFunds2QAgo"),
            date_2q_ago: cell_value(row, "NumberOfFundsDate2QAgo"),
            shares_held_3q_ago: cell_value(row, "NumberOfFunds3QAgo"),
            date_3q_ago: cell_value(row, "NumberOfFundsDate3QAgo"),
            shares_held_4q_ago: cell_value(row, "NumberOfFunds4QAgo"),
            date_4q_ago: cell_value(row, "NumberOfFundsDate4QAgo"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::common::test_support::response_value;
    use marketsurge_client::fundamentals::{
        FundamentalsInstrument, FundamentalsItem, FundamentalsSymbol, FundamentalsSymbology,
    };
    use marketsurge_client::ownership::{
        OwnershipData, OwnershipDateValue, OwnershipFormattedValue, OwnershipItem,
        OwnershipQuarterlySummary,
    };
    use marketsurge_client::screen::ResponseValue;

    use super::{cell_value, extract_dj_key, flatten_fund_rows, flatten_ownership_summary};

    #[test]
    fn test_cell_value_matching_value() {
        let row = vec![response_value("Symbol", Some("AAPL"))];

        assert_eq!(cell_value(&row, "Symbol"), Some("AAPL".to_string()));
    }

    #[test]
    fn test_cell_value_matching_empty_string() {
        let row = vec![response_value("Symbol", Some(""))];

        assert_eq!(cell_value(&row, "Symbol"), None);
    }

    #[test]
    fn test_cell_value_missing_column() {
        let row = vec![response_value("CompanyName", Some("Apple"))];

        assert_eq!(cell_value(&row, "Symbol"), None);
    }

    #[test]
    fn test_cell_value_empty_row() {
        let row: Vec<ResponseValue> = Vec::new();

        assert_eq!(cell_value(&row, "Symbol"), None);
    }

    // --- flatten_ownership_summary tests ---

    fn make_ownership_item(
        pct: Option<&str>,
        quarters: Vec<(Option<&str>, Option<&str>)>,
    ) -> OwnershipItem {
        OwnershipItem {
            ownership: Some(OwnershipData {
                funds_float_percent_held: pct.map(|v| OwnershipFormattedValue {
                    formatted_value: Some(v.to_string()),
                }),
                fund_ownership_summary: quarters
                    .into_iter()
                    .map(|(date, count)| OwnershipQuarterlySummary {
                        date: date.map(|d| OwnershipDateValue {
                            value: Some(d.to_string()),
                        }),
                        number_of_funds_held: count.map(|c| OwnershipFormattedValue {
                            formatted_value: Some(c.to_string()),
                        }),
                    })
                    .collect(),
            }),
        }
    }

    #[test]
    fn test_flatten_ownership_summary_with_quarters() {
        let items = vec![make_ownership_item(
            Some("62.3%"),
            vec![
                (Some("2026-03-31"), Some("5,432")),
                (Some("2025-12-31"), Some("5,210")),
            ],
        )];

        let records = flatten_ownership_summary(&["AAPL"], &items);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].funds_float_pct_held.as_deref(), Some("62.3%"));
        assert_eq!(records[0].date.as_deref(), Some("2026-03-31"));
        assert_eq!(records[0].num_funds_held.as_deref(), Some("5,432"));
        assert_eq!(records[1].date.as_deref(), Some("2025-12-31"));
        assert_eq!(records[1].num_funds_held.as_deref(), Some("5,210"));
    }

    #[test]
    fn test_flatten_ownership_summary_skips_none_and_empty() {
        let items = vec![
            // No ownership at all -> skipped
            OwnershipItem { ownership: None },
            // Ownership present but no quarters -> single record
            make_ownership_item(Some("10.0%"), vec![]),
        ];

        let records = flatten_ownership_summary(&["SKIP", "KEEP"], &items);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].symbol, "KEEP");
        assert_eq!(records[0].funds_float_pct_held.as_deref(), Some("10.0%"));
        assert!(records[0].date.is_none());
        assert!(records[0].num_funds_held.is_none());
    }

    // --- extract_dj_key tests ---

    fn make_fundamentals_item(symbols: Vec<(Option<&str>, Option<&str>)>) -> FundamentalsItem {
        FundamentalsItem {
            id: None,
            financials: None,
            symbology: Some(FundamentalsSymbology {
                company: None,
                instrument: Some(FundamentalsInstrument {
                    symbols: symbols
                        .into_iter()
                        .map(|(val, ntype)| FundamentalsSymbol {
                            value: val.map(str::to_string),
                            node_type: ntype.map(str::to_string),
                        })
                        .collect(),
                }),
            }),
        }
    }

    #[test]
    fn test_extract_dj_key_found() {
        let item = make_fundamentals_item(vec![
            (Some("AAPL"), Some("CHARTING")),
            (Some("XNAS-AAPL"), Some("DJ_KEY")),
        ]);

        assert_eq!(extract_dj_key(&item), Some("XNAS-AAPL"));
    }

    #[test]
    fn test_extract_dj_key_missing() {
        let item = make_fundamentals_item(vec![(Some("AAPL"), Some("CHARTING"))]);

        assert_eq!(extract_dj_key(&item), None);
    }

    #[test]
    fn test_extract_dj_key_no_symbology() {
        let item = FundamentalsItem {
            id: None,
            financials: None,
            symbology: None,
        };

        assert_eq!(extract_dj_key(&item), None);
    }

    // --- flatten_fund_rows tests ---

    #[test]
    fn test_flatten_fund_rows_two_rows() {
        let rows = vec![
            vec![
                response_value("Symbol", Some("VFIAX")),
                response_value("CompanyName", Some("Vanguard 500 Index")),
                response_value("HoldingsPctFundAssetsHeld", Some("6.82")),
                response_value("NumberOfFunds1QAgo", Some("1000")),
                response_value("NumberOfFundsDate1QAgo", Some("2026-03-31")),
            ],
            vec![
                response_value("Symbol", Some("FXAIX")),
                response_value("CompanyName", Some("Fidelity 500 Index")),
                response_value("HoldingsPctFundAssetsHeld", Some("5.10")),
                response_value("NumberOfFunds1QAgo", Some("900")),
                response_value("NumberOfFundsDate1QAgo", Some("2026-03-31")),
            ],
        ];

        let records = flatten_fund_rows("AAPL", &rows);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].queried_symbol, "AAPL");
        assert_eq!(records[0].fund_symbol.as_deref(), Some("VFIAX"));
        assert_eq!(records[0].fund_name.as_deref(), Some("Vanguard 500 Index"));
        assert_eq!(records[0].holdings_pct.as_deref(), Some("6.82"));
        assert_eq!(records[0].shares_held_1q_ago.as_deref(), Some("1000"));
        assert_eq!(records[0].date_1q_ago.as_deref(), Some("2026-03-31"));
        assert_eq!(records[1].queried_symbol, "AAPL");
        assert_eq!(records[1].fund_symbol.as_deref(), Some("FXAIX"));
        assert_eq!(records[1].fund_name.as_deref(), Some("Fidelity 500 Index"));
        assert_eq!(records[1].holdings_pct.as_deref(), Some("5.10"));
    }

    #[test]
    fn test_flatten_fund_rows_empty() {
        let rows: Vec<Vec<ResponseValue>> = Vec::new();

        let records = flatten_fund_rows("AAPL", &rows);

        assert!(records.is_empty());
    }

    #[test]
    fn test_flatten_fund_rows_missing_cell() {
        let rows = vec![vec![
            response_value("Symbol", Some("VFIAX")),
            // CompanyName deliberately absent
            response_value("HoldingsPctFundAssetsHeld", Some("6.82")),
        ]];

        let records = flatten_fund_rows("AAPL", &rows);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fund_symbol.as_deref(), Some("VFIAX"));
        assert!(records[0].fund_name.is_none());
        assert_eq!(records[0].holdings_pct.as_deref(), Some("6.82"));
        assert!(records[0].shares_held_1q_ago.is_none());
    }
}
