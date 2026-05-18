//! Stock screen commands for listing, running, and querying screens.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use serde::Serialize;
use tracing::instrument;

use crate::common::auth::handle_api_error;
use crate::common::command::run_client_command;

/// Screen subcommands.
#[derive(Debug, Subcommand)]
pub enum ScreenCommand {
    /// List screens. Use --coach to include predefined screens (e.g. IBD 50).
    List(ListArgs),
    /// Run a screen by ID or name and return matching instruments.
    Run(RunArgs),
}

/// Arguments for listing screens.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Include predefined coach screens (e.g. IBD 50, Recent Breakouts).
    #[arg(long)]
    pub coach: bool,
}

/// Arguments for running a saved screen.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Screen ID or name (e.g. "screen-Peter Lynch" or "IBD 50").
    pub screen_id: String,
    /// Maximum number of results to return.
    #[arg(long, default_value = "1000")]
    pub limit: i64,
}

/// Flat output record for a saved screen listing entry.
#[derive(Debug, Clone, Serialize)]
pub struct ScreenListRecord {
    /// Where this screen comes from ("user" or "coach").
    pub source: String,
    /// Screen identifier (use this with `screen run`).
    pub id: Option<String>,
    /// Screen name (can also be used with `screen run`).
    pub name: Option<String>,
    /// Screen type (e.g. "CUSTOM", "STOCK_SCREEN", "LEAF").
    pub screen_type: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Creation timestamp.
    pub created_at: Option<String>,
}

/// Handles the screen command group.
#[instrument(skip_all)]
pub async fn handle(args: &crate::cli::ScreenArgs, json_table: bool) -> i32 {
    match &args.command {
        ScreenCommand::List(a) => execute_list(a, json_table).await,
        ScreenCommand::Run(a) => execute_run(a, json_table).await,
    }
}

#[instrument(skip_all)]
async fn execute_list(args: &ListArgs, json_table: bool) -> i32 {
    let coach = args.coach;

    run_client_command(json_table, |client| async move {
        let mut records = Vec::new();

        // Always include user screens.
        let response = client
            .screens("marketsurge")
            .await
            .map_err(handle_api_error)?;

        for entry in response.user.iter().flat_map(|u| &u.screens) {
            records.push(ScreenListRecord {
                source: "user".to_string(),
                id: entry.id.clone(),
                name: entry.name.clone(),
                screen_type: entry.screen_type.clone(),
                description: entry.description.clone(),
                updated_at: entry.updated_at.clone(),
                created_at: entry.created_at.clone(),
            });
        }

        // Optionally include coach screens.
        if coach {
            let coach_response = client
                .coach_tree("marketsurge", "MSR_NAV")
                .await
                .map_err(handle_api_error)?;

            for node in coach_response
                .user
                .iter()
                .flat_map(|u| &u.screens)
                .filter(|n| n.reference_id.is_some())
            {
                records.push(ScreenListRecord {
                    source: "coach".to_string(),
                    id: node.reference_id.clone(),
                    name: node.name.clone(),
                    screen_type: node.node_type.clone(),
                    description: None,
                    updated_at: None,
                    created_at: None,
                });
            }
        }

        Ok(records)
    })
    .await
}

#[instrument(skip_all)]
async fn execute_run(args: &RunArgs, json_table: bool) -> i32 {
    let screen_id_or_name = args.screen_id.clone();
    let limit = args.limit;

    run_client_command(json_table, |client| async move {
        // Resolve name to ID via coach tree; falls back to input as-is.
        let screen_id = resolve_screen_id(&client, &screen_id_or_name).await;

        let input = marketsurge_client::screen::RunScreenInput {
            correlation_tag: "marketsurge".to_string(),
            coach_account: true,
            include_source: Some(marketsurge_client::screen::RunScreenIncludeSource {
                source: None,
            }),
            page_size: limit,
            result_limit: 1_000_000,
            screen_id,
            site: "marketsurge".to_string(),
            skip: 0,
            response_columns: Vec::new(),
        };

        let response = client.run_screen(input).await.map_err(handle_api_error)?;

        let records: Vec<BTreeMap<String, Option<String>>> = response
            .user
            .iter()
            .flat_map(|u| &u.run_screen)
            .flat_map(|result| &result.response_values)
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

/// Resolves a screen name to its ID by checking the coach tree.
///
/// If a matching coach screen name is found, returns its `referenceId`.
/// Otherwise returns the input unchanged (assumed to be a raw screen ID).
async fn resolve_screen_id(client: &marketsurge_client::Client, id_or_name: &str) -> String {
    let Ok(response) = client.coach_tree("marketsurge", "MSR_NAV").await else {
        return id_or_name.to_string();
    };

    response
        .user
        .iter()
        .flat_map(|u| &u.screens)
        .find(|node| node.name.as_deref() == Some(id_or_name))
        .and_then(|node| node.reference_id.clone())
        .unwrap_or_else(|| id_or_name.to_string())
}
