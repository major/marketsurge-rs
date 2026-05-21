//! Shared value wrapper types used across multiple endpoint modules.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default symbol dialect type for market data queries.
pub(crate) const DEFAULT_SYMBOL_DIALECT_TYPE: &str = "CHARTING";

/// Convert a borrowed symbol slice into owned strings for GraphQL variables.
pub(crate) fn symbols_to_owned(symbols: &[&str]) -> Vec<String> {
    symbols.iter().map(|s| (*s).to_string()).collect()
}

pub(crate) fn deserialize_first_array_element<T, E>(values: Vec<Value>) -> Result<Option<T>, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::Error,
{
    values
        .into_iter()
        .next()
        .map(serde_json::from_value)
        .transpose()
        .map_err(E::custom)
}

/// GraphQL variables for queries that only need symbols and dialect type.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolVariables {
    /// Ticker symbols.
    pub symbols: Vec<String>,
    /// Symbol dialect type (e.g. "CHARTING").
    pub symbol_dialect_type: String,
}

impl SymbolVariables {
    /// Creates new symbol variables with the given dialect, falling back to
    /// [`DEFAULT_SYMBOL_DIALECT_TYPE`] when `dialect` is `None`.
    pub fn new(symbols: &[&str], dialect: Option<&str>) -> Self {
        Self {
            symbols: symbols_to_owned(symbols),
            symbol_dialect_type: dialect.unwrap_or(DEFAULT_SYMBOL_DIALECT_TYPE).to_string(),
        }
    }
}

/// Sort specification for a response column.
///
/// Shared by screen, adhoc screen, and watchlist queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortInformation {
    /// Sort direction (e.g. "ASCENDING", "DESCENDING").
    pub direction: String,
    /// Sort priority order.
    pub order: String,
}

/// A response column to include in query results.
///
/// Shared by screen, adhoc screen, and watchlist queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseColumn {
    /// Column name.
    pub name: String,
    /// Optional sort specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_information: Option<SortInformation>,
}

/// Numeric value with optional formatted display string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormattedFloat {
    /// Raw numeric value.
    pub value: Option<f64>,
    /// Display-formatted string.
    pub formatted_value: Option<String>,
}

/// Single date string value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateValue {
    /// Date string (e.g. "2026-03-31").
    pub value: Option<String>,
}

/// Wrapper for a single numeric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatValue {
    /// Numeric value.
    pub value: Option<f64>,
}

/// A node in a tree hierarchy (either a folder or a leaf).
///
/// Folder-only fields (`children`, `content_type`) are absent for leaf nodes.
/// Leaf-only fields (`url`, `reference_id`) are absent for folders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    /// Node identifier.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Parent node identifier.
    pub parent_id: Option<String>,
    /// Node type (e.g. "SYSTEM_FOLDER", "REPORTS_SCREEN", "STOCK_SCREEN").
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    /// Child nodes (folders only).
    #[serde(default)]
    pub children: Vec<TreeChildNode>,
    /// Content type (e.g. "REPORTS", folders only).
    pub content_type: Option<String>,
    /// Tree type (e.g. "MSR_NAV").
    pub tree_type: Option<String>,
    /// URL path (leaves only).
    pub url: Option<String>,
    /// Reference identifier (leaves only, may be a JSON string).
    pub reference_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_SYMBOL_DIALECT_TYPE, SymbolVariables, symbols_to_owned};

    #[test]
    fn symbols_to_owned_preserves_order() {
        assert_eq!(symbols_to_owned(&["AAPL", "MSFT"]), vec!["AAPL", "MSFT"]);
    }

    #[test]
    fn symbol_variables_uses_default_dialect() {
        let variables = SymbolVariables::new(&["AAPL"], None);

        assert_eq!(variables.symbols, vec!["AAPL"]);
        assert_eq!(variables.symbol_dialect_type, DEFAULT_SYMBOL_DIALECT_TYPE);
    }

    #[test]
    fn symbol_variables_uses_custom_dialect() {
        let variables = SymbolVariables::new(&["AAPL"], Some("CUSTOM"));

        assert_eq!(variables.symbols, vec!["AAPL"]);
        assert_eq!(variables.symbol_dialect_type, "CUSTOM");
    }
}

/// A child node summary within a tree folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeChildNode {
    /// Node identifier.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Node type.
    #[serde(rename = "type")]
    pub node_type: Option<String>,
}
