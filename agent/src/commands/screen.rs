//! Stock screen commands for listing, running, and querying screens.

use clap::{Args, Subcommand};
use marketsurge_client::coach::{CoachTreeNode, CoachTreeResponse};
use marketsurge_client::screen::{ResponseValue, ScreenEntry, ScreensResponse};
use serde::Serialize;
use tracing::instrument;

use crate::common::command::{api_call, run_client_command};
use crate::common::rows::flatten_response_rows;

/// Screen subcommands.
#[derive(Debug, Subcommand)]
pub enum ScreenCommand {
    /// List user screens, optionally including predefined coach screens.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen list\n  marketsurge-agent screen list --coach"
    )]
    List(ListArgs),
    /// Run a screen by ID or name and return matching instruments.
    #[command(
        after_help = "Examples:\n  marketsurge-agent screen run 'IBD 50'\n  marketsurge-agent screen run 'screen-Peter Lynch' --limit 250"
    )]
    Run(RunArgs),
}

/// Arguments for listing screens.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Include predefined coach screens such as IBD 50.
    #[arg(long)]
    pub coach: bool,
}

/// Arguments for running a saved screen.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Screen ID or screen name, for example IBD 50.
    pub screen_id: String,
    /// Maximum rows returned.
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
#[cfg(not(coverage))]
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
        // Always include user screens.
        let screens_response = api_call(client.screens("marketsurge")).await?;

        // Optionally include coach screens.
        let coach_response = if coach {
            Some(api_call(client.coach_tree("marketsurge", "MSR_NAV")).await?)
        } else {
            None
        };

        Ok(flatten_screen_list(
            &screens_response,
            coach_response.as_ref(),
        ))
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

        let response = api_call(client.run_screen(input)).await?;

        let rows: &[Vec<ResponseValue>] = response
            .user
            .as_ref()
            .and_then(|u| u.run_screen.as_ref())
            .map(|result| result.response_values.as_slice())
            .unwrap_or(&[]);

        Ok(flatten_response_rows(rows))
    })
    .await
}

/// Assembles a flat list of screen records from user and optional coach responses.
///
/// User screens are always included. Coach screens are included only when
/// `coach_response` is `Some`, and only nodes with a `reference_id` are kept.
fn flatten_screen_list(
    screens_response: &ScreensResponse,
    coach_response: Option<&CoachTreeResponse>,
) -> Vec<ScreenListRecord> {
    let mut records = Vec::new();

    for entry in screens_response.user.iter().flat_map(|u| &u.screens) {
        records.push(map_user_screen_entry(entry));
    }

    if let Some(coach) = coach_response {
        for node in coach
            .user
            .iter()
            .flat_map(|u| &u.screens)
            .filter(|n| n.reference_id.is_some())
        {
            records.push(map_coach_screen_node(node));
        }
    }

    records
}

/// Maps a user screen entry to a flat output record.
fn map_user_screen_entry(entry: &ScreenEntry) -> ScreenListRecord {
    ScreenListRecord {
        source: "user".to_string(),
        id: entry.id.clone(),
        name: entry.name.clone(),
        screen_type: entry.screen_type.clone(),
        description: entry.description.clone(),
        updated_at: entry.updated_at.clone(),
        created_at: entry.created_at.clone(),
    }
}

/// Maps a coach tree node to a flat output record.
fn map_coach_screen_node(node: &CoachTreeNode) -> ScreenListRecord {
    ScreenListRecord {
        source: "coach".to_string(),
        id: node.reference_id.clone(),
        name: node.name.clone(),
        screen_type: node.node_type.clone(),
        description: None,
        updated_at: None,
        created_at: None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_support::{
        optional_response_value, response_value, response_value_without_md_item,
    };
    use marketsurge_client::coach::{CoachTreeResponse, CoachTreeUser};
    use marketsurge_client::screen::{ScreensResponse, ScreensUser};

    fn make_screen_entry(id: &str, name: &str) -> ScreenEntry {
        ScreenEntry {
            site: Some("marketsurge".to_string()),
            id: Some(id.to_string()),
            name: Some(name.to_string()),
            screen_type: Some("CUSTOM".to_string()),
            source: None,
            updated_at: Some("2025-01-15".to_string()),
            filter_criteria: None,
            description: Some("test screen".to_string()),
            created_at: Some("2025-01-01".to_string()),
        }
    }

    fn make_coach_node(name: &str, reference_id: &str) -> CoachTreeNode {
        CoachTreeNode {
            id: Some("node-1".to_string()),
            name: Some(name.to_string()),
            parent_id: None,
            node_type: Some("STOCK_SCREEN".to_string()),
            children: vec![],
            content_type: None,
            tree_type: Some("MSR_NAV".to_string()),
            url: None,
            reference_id: Some(reference_id.to_string()),
        }
    }

    #[test]
    fn map_user_screen_entry_sets_source_and_fields() {
        let entry = make_screen_entry("scr-42", "Growth Leaders");
        let record = map_user_screen_entry(&entry);

        assert_eq!(record.source, "user");
        assert_eq!(record.id.as_deref(), Some("scr-42"));
        assert_eq!(record.name.as_deref(), Some("Growth Leaders"));
        assert_eq!(record.screen_type.as_deref(), Some("CUSTOM"));
        assert_eq!(record.description.as_deref(), Some("test screen"));
        assert_eq!(record.updated_at.as_deref(), Some("2025-01-15"));
        assert_eq!(record.created_at.as_deref(), Some("2025-01-01"));
    }

    #[test]
    fn map_coach_screen_node_sets_source_and_fields() {
        let node = make_coach_node("IBD 50", "ref-ibd50");
        let record = map_coach_screen_node(&node);

        assert_eq!(record.source, "coach");
        assert_eq!(record.id.as_deref(), Some("ref-ibd50"));
        assert_eq!(record.name.as_deref(), Some("IBD 50"));
        assert_eq!(record.screen_type.as_deref(), Some("STOCK_SCREEN"));
        assert!(record.description.is_none());
        assert!(record.updated_at.is_none());
        assert!(record.created_at.is_none());
    }

    #[test]
    fn flatten_response_rows_converts_two_rows() {
        let rows = vec![
            vec![
                response_value("Symbol", Some("AAPL")),
                response_value("CompanyName", Some("Apple Inc")),
            ],
            vec![
                response_value("Symbol", Some("NVDA")),
                response_value("CompanyName", Some("NVIDIA Corp")),
            ],
        ];

        let result = flatten_response_rows(&rows);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("Symbol"), Some(&Some("AAPL".to_string())));
        assert_eq!(
            result[1].get("CompanyName"),
            Some(&Some("NVIDIA Corp".to_string()))
        );
    }

    #[test]
    fn flatten_response_rows_empty_input() {
        let result = flatten_response_rows(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn flatten_response_rows_skips_cells_without_md_item_name() {
        let rows = vec![vec![
            response_value("Symbol", Some("TSLA")),
            response_value_without_md_item(Some("ignored")),
            optional_response_value(None, Some("also-ignored")),
        ]];

        let result = flatten_response_rows(&rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0].get("Symbol"), Some(&Some("TSLA".to_string())));
    }

    fn make_screens_response(entries: Vec<ScreenEntry>) -> ScreensResponse {
        ScreensResponse {
            user: Some(ScreensUser { screens: entries }),
        }
    }

    fn make_coach_response(nodes: Vec<CoachTreeNode>) -> CoachTreeResponse {
        CoachTreeResponse {
            user: Some(CoachTreeUser {
                watchlists: vec![],
                screens: nodes,
            }),
        }
    }

    #[test]
    fn flatten_screen_list_user_only() {
        let resp = make_screens_response(vec![
            make_screen_entry("scr-1", "Growth"),
            make_screen_entry("scr-2", "Value"),
        ]);

        let records = flatten_screen_list(&resp, None);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].source, "user");
        assert_eq!(records[0].id.as_deref(), Some("scr-1"));
        assert_eq!(records[1].source, "user");
        assert_eq!(records[1].name.as_deref(), Some("Value"));
    }

    #[test]
    fn flatten_screen_list_with_coach() {
        let resp = make_screens_response(vec![make_screen_entry("scr-1", "My Screen")]);
        let coach = make_coach_response(vec![make_coach_node("IBD 50", "ref-ibd50")]);

        let records = flatten_screen_list(&resp, Some(&coach));

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].source, "user");
        assert_eq!(records[0].id.as_deref(), Some("scr-1"));
        assert_eq!(records[1].source, "coach");
        assert_eq!(records[1].id.as_deref(), Some("ref-ibd50"));
        assert_eq!(records[1].name.as_deref(), Some("IBD 50"));
    }

    #[test]
    fn flatten_screen_list_skips_coach_without_reference_id() {
        let resp = make_screens_response(vec![]);
        let coach = make_coach_response(vec![
            make_coach_node("IBD 50", "ref-ibd50"),
            CoachTreeNode {
                id: Some("node-2".to_string()),
                name: Some("Folder Node".to_string()),
                parent_id: None,
                node_type: Some("FOLDER".to_string()),
                children: vec![],
                content_type: None,
                tree_type: None,
                url: None,
                reference_id: None,
            },
        ]);

        let records = flatten_screen_list(&resp, Some(&coach));

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "coach");
        assert_eq!(records[0].id.as_deref(), Some("ref-ibd50"));
    }
}
