//! Helpers for tabular API response rows.

use std::collections::BTreeMap;

use crate::screen::ResponseValue;
use crate::types::ResponseColumn;

/// Converts response rows into flat key-value maps.
pub(crate) fn flatten_response_rows(
    response_values: &[Vec<ResponseValue>],
) -> Vec<BTreeMap<String, Option<String>>> {
    response_values
        .iter()
        .map(|row| {
            row.iter()
                .filter_map(|cell| {
                    let name = cell.md_item.as_ref().and_then(|m| m.name.clone())?;
                    Some((name, cell.value.clone()))
                })
                .collect()
        })
        .collect()
}

/// Builds response column requests with no sort information.
pub(crate) fn response_columns(names: &[String]) -> Vec<ResponseColumn> {
    names
        .iter()
        .map(|name| ResponseColumn {
            name: name.clone(),
            sort_information: None,
        })
        .collect()
}

/// Truncates records to at most `limit` entries.
///
/// A limit of zero means no limit.
pub(crate) fn truncate_records<T>(mut records: Vec<T>, limit: usize) -> Vec<T> {
    if limit > 0 && records.len() > limit {
        records.truncate(limit);
    }

    records
}

#[cfg(test)]
mod tests {
    use super::{flatten_response_rows, response_columns, truncate_records};
    use crate::cli::common::test_support::{
        optional_response_value, response_value, response_value_without_md_item,
    };

    #[test]
    fn flatten_response_rows_converts_named_cells() {
        let rows = vec![
            vec![
                response_value("Symbol", Some("AAPL")),
                response_value("RS", None),
            ],
            vec![
                response_value("Symbol", Some("MSFT")),
                response_value("RS", Some("95")),
            ],
        ];

        let flattened = flatten_response_rows(&rows);

        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].get("Symbol"), Some(&Some("AAPL".to_string())));
        assert_eq!(flattened[0].get("RS"), Some(&None));
        assert_eq!(flattened[1].get("Symbol"), Some(&Some("MSFT".to_string())));
        assert_eq!(flattened[1].get("RS"), Some(&Some("95".to_string())));
    }

    #[test]
    fn flatten_response_rows_skips_cells_without_names() {
        let rows = vec![vec![
            response_value("Symbol", Some("AAPL")),
            optional_response_value(None, Some("ignored")),
            response_value_without_md_item(Some("also ignored")),
        ]];

        let flattened = flatten_response_rows(&rows);

        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].len(), 1);
        assert_eq!(flattened[0].get("Symbol"), Some(&Some("AAPL".to_string())));
    }

    #[test]
    fn flatten_response_rows_handles_empty_input() {
        let rows = Vec::new();

        let flattened = flatten_response_rows(&rows);

        assert!(flattened.is_empty());
    }

    #[test]
    fn response_columns_sets_sort_information_none() {
        let names = vec!["Symbol".to_string(), "RSRating".to_string()];

        let columns = response_columns(&names);

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "Symbol");
        assert!(columns[0].sort_information.is_none());
        assert_eq!(columns[1].name, "RSRating");
        assert!(columns[1].sort_information.is_none());
    }

    #[test]
    fn truncate_records_keeps_all_records_when_limit_is_zero() {
        let records = vec![1, 2, 3];

        assert_eq!(truncate_records(records, 0), vec![1, 2, 3]);
    }

    #[test]
    fn truncate_records_keeps_all_records_when_limit_exceeds_len() {
        let records = vec![1, 2, 3];

        assert_eq!(truncate_records(records, 5), vec![1, 2, 3]);
    }

    #[test]
    fn truncate_records_keeps_all_records_when_limit_matches_len() {
        let records = vec![1, 2, 3];

        assert_eq!(truncate_records(records, 3), vec![1, 2, 3]);
    }

    #[test]
    fn truncate_records_keeps_first_limit_records() {
        let records = vec![1, 2, 3];

        assert_eq!(truncate_records(records, 2), vec![1, 2]);
    }

    #[test]
    fn truncate_records_handles_empty_records() {
        let records: Vec<i32> = Vec::new();

        assert!(truncate_records(records, 2).is_empty());
    }
}
