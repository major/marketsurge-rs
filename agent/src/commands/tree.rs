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

        let mut records = Vec::new();

        if let Some(user) = &response.user {
            for node in &user.watchlists {
                records.push(node_to_record("watchlist", node));
            }
            for node in &user.screens {
                records.push(node_to_record("screen", node));
            }
        }

        Ok(records)
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

        let records: Vec<TreeRecord> = response
            .user
            .iter()
            .flat_map(|u| &u.nav_tree)
            .map(|node| node_to_record("nav", node))
            .collect();

        Ok(records)
    })
    .await
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
