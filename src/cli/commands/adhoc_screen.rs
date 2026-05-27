//! Ad-hoc stock screening command.

use crate::adhoc_screen::{AdhocScreenId, AdhocScreenIncludeSource, AdhocScreenInstruments};
use clap::Args;
use std::collections::BTreeSet;
use tracing::instrument;

use crate::cli::common::command::{api_call, run_client_command};
use crate::cli::common::error::{render_usage_message, render_warning_message};
use crate::cli::common::rows::{flatten_response_rows, response_columns};
use crate::screen::ResponseValue;

const INVALID_COLUMN_SUGGESTION: &str = "Check the column names and spelling before retrying.";

/// Arguments for the adhoc-screen command.
#[derive(Debug, Args)]
pub struct AdhocScreenCommandArgs {
    /// Output columns, comma-separated.
    #[arg(long, value_delimiter = ',', default_value = "Symbol,CompanyName")]
    pub columns: Vec<String>,

    /// Screener filter as raw JSON.
    #[arg(long)]
    pub query: Option<String>,

    /// Use symbols from this saved screen ID.
    #[arg(long, conflicts_with = "symbols")]
    pub screen_id: Option<i64>,

    /// Screen these comma-separated ticker symbols.
    #[arg(long, value_delimiter = ',', conflicts_with = "screen_id")]
    pub symbols: Option<Vec<String>>,

    /// Symbol dialect. Defaults depend on --screen-id or --symbols.
    #[arg(long)]
    pub dialect: Option<String>,

    /// API page size.
    #[arg(long, default_value = "1000")]
    pub page_size: i64,

    /// Maximum rows returned.
    #[arg(long, default_value = "1000000")]
    pub limit: i64,

    /// Rows to skip before returning results.
    #[arg(long, default_value = "0")]
    pub skip: i64,
}

/// Handles the adhoc-screen command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &AdhocScreenCommandArgs, fields: &[String]) -> i32 {
    let requested_columns = normalized_columns(&args.columns);
    if requested_columns.is_empty() {
        return render_usage_message("--columns must include at least one column name".to_string());
    }
    let columns = response_columns(&requested_columns);

    let adhoc_query: Option<serde_json::Value> = match &args.query {
        Some(q) => match serde_json::from_str(q) {
            Ok(v) => Some(v),
            Err(e) => {
                return render_usage_message(format!("invalid --query JSON: {e}"));
            }
        },
        None => None,
    };

    let include_source = build_include_source(args);

    let page_size = args.page_size;
    let result_limit = args.limit;
    let page_skip = args.skip;

    run_client_command(fields, |client| async move {
        let response = api_call(client.market_data_adhoc_screen(
            "marketsurge",
            columns,
            adhoc_query,
            include_source,
            page_size,
            result_limit,
            page_skip,
            "RESULT_WITH_EXPRESSION_COUNTS",
        ))
        .await?;

        let response_values = response
            .market_data_adhoc_screen
            .as_ref()
            .map(|result| &result.response_values[..])
            .unwrap_or(&[]);

        let invalid_columns = missing_columns(&requested_columns, response_values);
        if invalid_columns.len() == requested_columns.len() {
            return Err(render_usage_message(all_invalid_columns_message(
                &invalid_columns,
            )));
        }

        if !invalid_columns.is_empty() {
            render_warning_message(
                partial_invalid_columns_message(&invalid_columns),
                Some(INVALID_COLUMN_SUGGESTION.to_string()),
            );
        }

        Ok(flatten_response_rows(response_values))
    })
    .await
}

fn normalized_columns(columns: &[String]) -> Vec<String> {
    columns
        .iter()
        .map(|column| column.trim())
        .filter(|column| !column.is_empty())
        .map(str::to_string)
        .collect()
}

fn missing_columns(
    requested_columns: &[String],
    response_values: &[Vec<ResponseValue>],
) -> Vec<String> {
    if response_values.is_empty() {
        return Vec::new();
    }

    let returned_columns: BTreeSet<&str> = response_values
        .iter()
        .flat_map(|row| row.iter())
        .filter_map(|cell| cell.md_item.as_ref()?.name.as_deref())
        .collect();

    requested_columns
        .iter()
        .filter(|column| !returned_columns.contains(column.as_str()))
        .cloned()
        .collect()
}

fn all_invalid_columns_message(columns: &[String]) -> String {
    format!(
        "unrecognized screen adhoc column name(s): {}",
        columns.join(", ")
    )
}

fn partial_invalid_columns_message(columns: &[String]) -> String {
    format!(
        "unrecognized screen adhoc column name(s): {}. Valid columns in the same request were still returned.",
        columns.join(", ")
    )
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

#[cfg(test)]
mod tests {
    use super::{
        AdhocScreenCommandArgs, all_invalid_columns_message, build_include_source, missing_columns,
        normalized_columns, partial_invalid_columns_message,
    };
    use crate::cli::common::rows::flatten_response_rows;
    use crate::cli::common::test_support::{
        optional_response_value, response_value, response_value_without_md_item,
    };

    fn args(
        screen_id: Option<i64>,
        symbols: Option<Vec<String>>,
        dialect: Option<&str>,
    ) -> AdhocScreenCommandArgs {
        AdhocScreenCommandArgs {
            columns: vec!["Symbol".to_string(), "CompanyName".to_string()],
            query: None,
            screen_id,
            symbols,
            dialect: dialect.map(str::to_string),
            page_size: 1000,
            limit: 1_000_000,
            skip: 0,
        }
    }

    #[test]
    fn test_build_include_source_empty_args() {
        let include_source = build_include_source(&args(None, None, None));

        assert!(include_source.screen_id.is_none());
        assert!(include_source.instruments.is_none());
    }

    #[test]
    fn test_build_include_source_screen_id_defaults_dialect() {
        let include_source = build_include_source(&args(Some(123), None, None));

        let screen_id = include_source.screen_id.as_ref().expect("screen_id");
        assert_eq!(screen_id.id, 123);
        assert_eq!(screen_id.dialect, "MS_LIST_ID");
        assert!(include_source.instruments.is_none());
    }

    #[test]
    fn test_build_include_source_symbols_defaults_dialect() {
        let include_source =
            build_include_source(&args(None, Some(vec!["AAPL".to_string()]), None));

        let instruments = include_source.instruments.as_ref().expect("instruments");
        assert_eq!(instruments.symbols, vec!["AAPL".to_string()]);
        assert_eq!(instruments.dialect, "CHARTING");
        assert!(include_source.screen_id.is_none());
    }

    #[test]
    fn test_build_include_source_screen_id_takes_precedence() {
        let include_source =
            build_include_source(&args(Some(123), Some(vec!["AAPL".to_string()]), None));

        let screen_id = include_source.screen_id.as_ref().expect("screen_id");
        assert_eq!(screen_id.id, 123);
        assert_eq!(screen_id.dialect, "MS_LIST_ID");
        assert!(include_source.instruments.is_none());
    }

    #[test]
    fn test_build_include_source_symbols_uses_custom_dialect() {
        let include_source =
            build_include_source(&args(None, Some(vec!["AAPL".to_string()]), Some("CUSTOM")));

        let instruments = include_source.instruments.as_ref().expect("instruments");
        assert_eq!(instruments.symbols, vec!["AAPL".to_string()]);
        assert_eq!(instruments.dialect, "CUSTOM");
        assert!(include_source.screen_id.is_none());
    }

    #[test]
    fn test_flatten_multiple_rows() {
        let rows = vec![
            vec![
                response_value("Symbol", Some("AAPL")),
                response_value("RS", Some("95")),
            ],
            vec![
                response_value("Symbol", Some("NVDA")),
                response_value("RS", Some("99")),
            ],
            vec![
                response_value("Symbol", Some("TSLA")),
                response_value("RS", Some("80")),
            ],
        ];

        let result = flatten_response_rows(&rows);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].get("Symbol").unwrap(), &Some("AAPL".to_string()));
        assert_eq!(result[0].get("RS").unwrap(), &Some("95".to_string()));
        assert_eq!(result[1].get("Symbol").unwrap(), &Some("NVDA".to_string()));
        assert_eq!(result[2].get("Symbol").unwrap(), &Some("TSLA".to_string()));
    }

    #[test]
    fn test_flatten_empty_input() {
        let rows = vec![];
        let result = flatten_response_rows(&rows);
        assert!(result.is_empty());
    }

    #[test]
    fn test_flatten_missing_md_item_name_omits_key() {
        let rows = vec![vec![
            response_value("Symbol", Some("AAPL")),
            optional_response_value(None, Some("orphan_value")),
            response_value_without_md_item(Some("no_metadata")),
        ]];

        let result = flatten_response_rows(&rows);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0].get("Symbol").unwrap(), &Some("AAPL".to_string()));
    }

    #[test]
    fn test_normalized_columns_trims_and_omits_empty_names() {
        let columns = vec![
            " Symbol ".to_string(),
            String::new(),
            "RSRating".to_string(),
            "  ".to_string(),
        ];

        assert_eq!(normalized_columns(&columns), vec!["Symbol", "RSRating"]);
    }

    #[test]
    fn test_missing_columns_reports_requested_names_absent_from_response() {
        let requested = vec![
            "Symbol".to_string(),
            "FakeColumn".to_string(),
            "RSRating".to_string(),
        ];
        let rows = vec![vec![
            response_value("Symbol", Some("AAPL")),
            response_value("RSRating", Some("92")),
        ]];

        assert_eq!(missing_columns(&requested, &rows), vec!["FakeColumn"]);
    }

    #[test]
    fn test_missing_columns_looks_across_all_rows() {
        let requested = vec!["Symbol".to_string(), "RSRating".to_string()];
        let rows = vec![
            vec![response_value("Symbol", Some("AAPL"))],
            vec![response_value("RSRating", Some("92"))],
        ];

        assert!(missing_columns(&requested, &rows).is_empty());
    }

    #[test]
    fn test_missing_columns_does_not_warn_for_empty_results() {
        let requested = vec!["Symbol".to_string(), "RSRating".to_string()];

        assert!(missing_columns(&requested, &[]).is_empty());
    }

    #[test]
    fn test_partial_invalid_columns_message_lists_invalid_names() {
        let message =
            partial_invalid_columns_message(&["FakeColumn".to_string(), "RSrating".to_string()]);

        assert!(message.contains("FakeColumn, RSrating"));
        assert!(message.contains("Valid columns in the same request were still returned"));
    }

    #[test]
    fn test_all_invalid_columns_message_lists_invalid_names_without_partial_success() {
        let message =
            all_invalid_columns_message(&["FakeColumn".to_string(), "RSrating".to_string()]);

        assert!(message.contains("FakeColumn, RSrating"));
        assert!(!message.contains("Valid columns in the same request were still returned"));
    }
}
