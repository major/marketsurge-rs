//! Navigation and coaching tree commands.

use clap::Subcommand;
use serde::Serialize;
use tracing::instrument;

use crate::cli::TreeArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::run_client_command;

/// Tree subcommands.
#[derive(Debug, Subcommand)]
pub enum TreeCommand {
    /// Fetch the coach tree (watchlists and screens).
    Coach,
    /// Fetch the navigation tree.
    Nav,
}

/// Flat output record for a tree node.
#[derive(Debug, Clone, Serialize)]
pub struct TreeRecord {
    /// Source category (e.g. "watchlist", "screen", "nav").
    pub source: String,
    /// Node identifier.
    pub id: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Parent node identifier.
    pub parent_id: Option<String>,
    /// Node type (e.g. "SYSTEM_FOLDER", "STOCK_SCREEN").
    pub node_type: Option<String>,
    /// Content type (folders only, e.g. "REPORTS").
    pub content_type: Option<String>,
    /// Tree type (e.g. "MSR_NAV").
    pub tree_type: Option<String>,
    /// URL path (leaves only).
    pub url: Option<String>,
    /// Reference identifier (leaves only).
    pub reference_id: Option<String>,
}

/// Handles the tree command group.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &TreeArgs, json_table: bool) -> i32 {
    match &args.command {
        TreeCommand::Coach => execute_coach(json_table).await,
        TreeCommand::Nav => execute_nav(json_table).await,
    }
}

#[instrument(skip_all)]
async fn execute_coach(json_table: bool) -> i32 {
    run_client_command(json_table, |client| async move {
        let response = client
            .coach_tree("marketsurge", "MSR_NAV")
            .await
            .map_err(handle_api_error)?;

        Ok(flatten_coach_tree(&response))
    })
    .await
}

#[instrument(skip_all)]
async fn execute_nav(json_table: bool) -> i32 {
    run_client_command(json_table, |client| async move {
        let response = client
            .nav_tree("marketsurge", "MSR_NAV")
            .await
            .map_err(handle_api_error)?;

        Ok(flatten_nav_tree(&response))
    })
    .await
}

/// Flattens a coach tree response into a list of records.
///
/// Watchlist nodes get source `"watchlist"`, screen nodes get `"screen"`.
/// Returns an empty `Vec` when the response has no user data.
fn flatten_coach_tree(response: &marketsurge_client::coach::CoachTreeResponse) -> Vec<TreeRecord> {
    let Some(user) = &response.user else {
        return Vec::new();
    };

    let mut records = Vec::new();
    for node in &user.watchlists {
        records.push(node_to_record("watchlist", node));
    }
    for node in &user.screens {
        records.push(node_to_record("screen", node));
    }
    records
}

/// Flattens a nav tree response into a list of records.
///
/// All nodes get source `"nav"`.
/// Returns an empty `Vec` when the response has no user data.
fn flatten_nav_tree(response: &marketsurge_client::nav::NavTreeResponse) -> Vec<TreeRecord> {
    response
        .user
        .iter()
        .flat_map(|u| &u.nav_tree)
        .map(|node| node_to_record("nav", node))
        .collect()
}

fn node_to_record(source: &str, node: &marketsurge_client::types::TreeNode) -> TreeRecord {
    TreeRecord {
        source: source.to_string(),
        id: node.id.clone(),
        name: node.name.clone(),
        parent_id: node.parent_id.clone(),
        node_type: node.node_type.clone(),
        content_type: node.content_type.clone(),
        tree_type: node.tree_type.clone(),
        url: node.url.clone(),
        reference_id: node.reference_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketsurge_client::coach::{CoachTreeResponse, CoachTreeUser};
    use marketsurge_client::nav::{NavTreeResponse, NavTreeUser};
    use marketsurge_client::types::{TreeChildNode, TreeNode};

    #[test]
    fn node_to_record_all_fields_populated() {
        let node = TreeNode {
            id: Some("123".to_string()),
            name: Some("My Watchlist".to_string()),
            parent_id: Some("0".to_string()),
            node_type: Some("SYSTEM_FOLDER".to_string()),
            children: vec![TreeChildNode {
                id: Some("456".to_string()),
                name: Some("Child".to_string()),
                node_type: Some("STOCK_SCREEN".to_string()),
            }],
            content_type: Some("REPORTS".to_string()),
            tree_type: Some("MSR_NAV".to_string()),
            url: Some("/reports/123".to_string()),
            reference_id: Some("ref-abc".to_string()),
        };

        let record = node_to_record("watchlist", &node);

        assert_eq!(record.source, "watchlist");
        assert_eq!(record.id.as_deref(), Some("123"));
        assert_eq!(record.name.as_deref(), Some("My Watchlist"));
        assert_eq!(record.parent_id.as_deref(), Some("0"));
        assert_eq!(record.node_type.as_deref(), Some("SYSTEM_FOLDER"));
        assert_eq!(record.content_type.as_deref(), Some("REPORTS"));
        assert_eq!(record.tree_type.as_deref(), Some("MSR_NAV"));
        assert_eq!(record.url.as_deref(), Some("/reports/123"));
        assert_eq!(record.reference_id.as_deref(), Some("ref-abc"));
    }

    #[test]
    fn node_to_record_all_optional_fields_none() {
        let node = TreeNode {
            id: None,
            name: None,
            parent_id: None,
            node_type: None,
            children: vec![],
            content_type: None,
            tree_type: None,
            url: None,
            reference_id: None,
        };

        let record = node_to_record("screen", &node);

        assert_eq!(record.source, "screen");
        assert!(record.id.is_none());
        assert!(record.name.is_none());
        assert!(record.parent_id.is_none());
        assert!(record.node_type.is_none());
        assert!(record.content_type.is_none());
        assert!(record.tree_type.is_none());
        assert!(record.url.is_none());
        assert!(record.reference_id.is_none());
    }

    #[test]
    fn node_to_record_source_string_mapping() {
        let node = TreeNode {
            id: Some("1".to_string()),
            name: None,
            parent_id: None,
            node_type: None,
            children: vec![],
            content_type: None,
            tree_type: None,
            url: None,
            reference_id: None,
        };

        let watchlist = node_to_record("watchlist", &node);
        assert_eq!(watchlist.source, "watchlist");

        let screen = node_to_record("screen", &node);
        assert_eq!(screen.source, "screen");

        let nav = node_to_record("nav", &node);
        assert_eq!(nav.source, "nav");
    }

    fn make_tree_node(id: &str, name: &str) -> TreeNode {
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

    #[test]
    fn flatten_coach_tree_mixed_watchlists_and_screens() {
        let response = CoachTreeResponse {
            user: Some(CoachTreeUser {
                watchlists: vec![
                    make_tree_node("w1", "Watchlist 1"),
                    make_tree_node("w2", "Watchlist 2"),
                ],
                screens: vec![make_tree_node("s1", "Screen 1")],
            }),
        };

        let records = flatten_coach_tree(&response);

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].source, "watchlist");
        assert_eq!(records[0].id.as_deref(), Some("w1"));
        assert_eq!(records[1].source, "watchlist");
        assert_eq!(records[1].id.as_deref(), Some("w2"));
        assert_eq!(records[2].source, "screen");
        assert_eq!(records[2].id.as_deref(), Some("s1"));
    }

    #[test]
    fn flatten_coach_tree_none_user_returns_empty() {
        let response = CoachTreeResponse { user: None };

        let records = flatten_coach_tree(&response);

        assert!(records.is_empty());
    }

    #[test]
    fn flatten_nav_tree_multiple_nodes() {
        let response = NavTreeResponse {
            user: Some(NavTreeUser {
                nav_tree: vec![make_tree_node("n1", "Nav 1"), make_tree_node("n2", "Nav 2")],
            }),
        };

        let records = flatten_nav_tree(&response);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].source, "nav");
        assert_eq!(records[0].id.as_deref(), Some("n1"));
        assert_eq!(records[1].source, "nav");
        assert_eq!(records[1].id.as_deref(), Some("n2"));
    }

    #[test]
    fn flatten_nav_tree_none_user_returns_empty() {
        let response = NavTreeResponse { user: None };

        let records = flatten_nav_tree(&response);

        assert!(records.is_empty());
    }
}
