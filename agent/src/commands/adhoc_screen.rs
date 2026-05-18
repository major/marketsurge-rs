//! Ad-hoc stock screening command.

use std::collections::BTreeMap;

use clap::Args;
use marketsurge_client::adhoc_screen::{
    AdhocScreenId, AdhocScreenIncludeSource, AdhocScreenInstruments,
};
use marketsurge_client::types::ResponseColumn;
use tracing::instrument;

use crate::common::auth::handle_api_error;
use crate::common::command::run_client_command;

/// Arguments for the adhoc-screen command.
#[derive(Debug, Args)]
pub struct AdhocScreenCommandArgs {
    /// Data columns to include in results (comma-separated).
    #[arg(long, value_delimiter = ',', default_value = "Symbol,CompanyName")]
    pub columns: Vec<String>,

    /// JSON adhoc query filter string.
    #[arg(long)]
    pub query: Option<String>,

    /// Predefined screen ID to use as instrument source.
    #[arg(long, conflicts_with = "symbols")]
    pub screen_id: Option<i64>,

    /// Ticker symbols to screen (comma-separated, alternative to --screen-id).
    #[arg(long, value_delimiter = ',', conflicts_with = "screen_id")]
    pub symbols: Option<Vec<String>>,

    /// Symbol dialect (defaults to "MS_LIST_ID" for --screen-id, "CHARTING" for --symbols).
    #[arg(long)]
    pub dialect: Option<String>,

    /// Maximum results per page.
    #[arg(long, default_value = "1000")]
    pub page_size: i64,

    /// Maximum total results.
    #[arg(long, default_value = "1000000")]
    pub limit: i64,

    /// Number of results to skip.
    #[arg(long, default_value = "0")]
    pub skip: i64,
}

/// Handles the adhoc-screen command.
#[instrument(skip_all)]
pub async fn handle(args: &AdhocScreenCommandArgs, json_table: bool) -> i32 {
    let columns: Vec<ResponseColumn> = args
        .columns
        .iter()
        .map(|name| ResponseColumn {
            name: name.clone(),
            sort_information: None,
        })
        .collect();

    let adhoc_query: Option<serde_json::Value> = match &args.query {
        Some(q) => match serde_json::from_str(q) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!("invalid --query JSON: {e}");
                return 1;
            }
        },
        None => None,
    };

    let include_source = build_include_source(args);

    let page_size = args.page_size;
    let result_limit = args.limit;
    let page_skip = args.skip;

    run_client_command(json_table, |client| async move {
        let response = client
            .market_data_adhoc_screen(
                "marketsurge",
                columns,
                adhoc_query,
                include_source,
                page_size,
                result_limit,
                page_skip,
                "RESULT_WITH_EXPRESSION_COUNTS",
            )
            .await
            .map_err(handle_api_error)?;

        let records: Vec<BTreeMap<String, Option<String>>> = response
            .market_data_adhoc_screen
            .as_ref()
            .map(|result| &result.response_values)
            .unwrap_or(&Vec::new())
            .iter()
            .map(|row| {
                row.iter()
                    .filter_map(|cell| {
                        let name = cell.md_item.as_ref().and_then(|m| m.name.clone())?;
                        Some((name, cell.value.clone()))
                    })
                    .collect()
            })
            .collect();

        Ok(records)
    })
    .await
}

fn build_include_source(args: &AdhocScreenCommandArgs) -> AdhocScreenIncludeSource {
    if let Some(id) = args.screen_id {
        let dialect = args
            .dialect
            .clone()
            .unwrap_or_else(|| "MS_LIST_ID".to_string());
        AdhocScreenIncludeSource {
            screen_id: Some(AdhocScreenId { id, dialect }),
            instruments: None,
        }
    } else if let Some(symbols) = &args.symbols {
        let dialect = args
            .dialect
            .clone()
            .unwrap_or_else(|| "CHARTING".to_string());
        AdhocScreenIncludeSource {
            screen_id: None,
            instruments: Some(AdhocScreenInstruments {
                symbols: symbols.clone(),
                dialect,
            }),
        }
    } else {
        AdhocScreenIncludeSource {
            screen_id: None,
            instruments: None,
        }
    }
}
