//! Fund ownership data commands.

use clap::Subcommand;
use marketsurge_client::screen::{ResponseValue, ScreenerParameter};
use serde::Serialize;
use tracing::instrument;

use crate::cli::{OwnershipArgs, SymbolsArgs};
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

/// Screen name for the fund ownership detail query.
const FUND_OWNERSHIP_SCREEN: &str = "MarketSurge.RelatedInformation.MUTIFundOwnership";

/// Ownership subcommands.
#[derive(Debug, Subcommand)]
pub enum OwnershipCommand {
    /// Fetch fund ownership summary for one or more symbols.
    Summary(SymbolsArgs),
    /// Fetch individual fund holdings for one or more symbols.
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
pub async fn handle(args: &OwnershipArgs, json_table: bool) -> i32 {
    match &args.command {
        OwnershipCommand::Summary(a) => execute_summary(a, json_table).await,
        OwnershipCommand::Funds(a) => execute_funds(a, json_table).await,
    }
}

#[instrument(skip_all)]
async fn execute_summary(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .ownership(&symbol_refs)
                .await
                .map_err(handle_api_error)?;

            let mut records = Vec::new();
            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
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

            Ok(records)
        },
    )
    .await
}

#[instrument(skip_all)]
async fn execute_funds(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            // Resolve DJ_KEY for each symbol via the fundamentals API symbology.
            let fundamentals = client
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

            for (symbol, item) in zip_symbols(&symbol_refs, &fundamentals.market_data) {
                // Extract DJ_KEY from symbology and split into exchange + id.
                let dj_key = item
                    .symbology
                    .as_ref()
                    .and_then(|s| s.instrument.as_ref())
                    .map(|inst| &inst.symbols)
                    .and_then(|symbols| {
                        symbols
                            .iter()
                            .find(|s| s.node_type.as_deref() == Some("DJ_KEY"))
                    })
                    .and_then(|s| s.value.as_deref());

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

                let response = client
                    .market_data_screen(FUND_OWNERSHIP_SCREEN, parameters)
                    .await
                    .map_err(handle_api_error)?;

                if let Some(result) = response.market_data_screen {
                    for row in &result.response_values {
                        records.push(FundOwnershipRecord {
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
                        });
                    }
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

#[cfg(test)]
mod tests {
    use marketsurge_client::screen::{MdItem, ResponseValue};

    use super::cell_value;

    fn response_value(name: &str, value: Option<&str>) -> ResponseValue {
        ResponseValue {
            value: value.map(str::to_string),
            md_item: Some(MdItem {
                md_item_id: None,
                name: Some(name.to_string()),
            }),
        }
    }

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
}
