//! Test helpers shared across command modules.

use crate::screen::{MdItem, ResponseValue};
use crate::types::TreeNode;

/// Builds a named screen response value for command flattening tests.
pub(crate) fn response_value(name: &str, value: Option<&str>) -> ResponseValue {
    optional_response_value(Some(name), value)
}

/// Builds a screen response value whose metadata name may be absent.
pub(crate) fn optional_response_value(name: Option<&str>, value: Option<&str>) -> ResponseValue {
    ResponseValue {
        value: value.map(str::to_string),
        md_item: Some(MdItem {
            md_item_id: None,
            name: name.map(str::to_string),
        }),
    }
}

/// Builds a screen response value with no metadata item.
pub(crate) fn response_value_without_md_item(value: Option<&str>) -> ResponseValue {
    ResponseValue {
        value: value.map(str::to_string),
        md_item: None,
    }
}

/// Builds a simple tree node with only ID and name populated.
pub(crate) fn tree_node(id: &str, name: &str) -> TreeNode {
    TreeNode {
        id: Some(id.to_string()),
        name: Some(name.to_string()),
        parent_id: None,
        node_type: None,
        children: vec![],
        content_type: None,
        tree_type: None,
        url: None,
        reference_id: None,
    }
}
