//! Helpers for tabular API response rows.

use std::collections::BTreeMap;

use marketsurge_client::screen::ResponseValue;
use marketsurge_client::types::ResponseColumn;

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
