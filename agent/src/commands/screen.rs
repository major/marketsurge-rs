//! Stock screen commands for listing, running, and querying screens.

use clap::{Args, Subcommand};
use marketsurge_client::ClientError;
use marketsurge_client::coach::{CoachTreeNode, CoachTreeResponse};
use marketsurge_client::screen::{ResponseValue, ScreenEntry, ScreensResponse};
use serde::Serialize;
use tracing::{error, instrument};

use crate::common::command::{api_call, run_client_command};
use crate::common::rows::flatten_response_rows;

const IBD_50_LIMITATION: &str = "MarketSurge did not expose an official IBD 50 screen in the coach tree. Try `screen list --coach` or `tree coach` for currently exposed lists such as EF-50.";

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
#[cfg(not(coverage))]
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
#[cfg(not(coverage))]
async fn execute_run(args: &RunArgs, json_table: bool) -> i32 {
    let screen_id_or_name = args.screen_id.clone();
    let limit = args.limit;

    run_client_command(json_table, |client| async move {
        // Resolve name to ID via coach tree; falls back to input as-is.
        let Some(screen_id) = resolve_screen_id(&client, &screen_id_or_name).await? else {
            error!("{IBD_50_LIMITATION}");
            return Err(1);
        };

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

        let response = match client.run_screen(input).await {
            Ok(response) => response,
            Err(err) if is_ibd_50_name(&screen_id_or_name) && is_not_found_error(&err) => {
                error!("{IBD_50_LIMITATION}");
                return Err(1);
            }
            Err(err) => return Err(crate::common::auth::handle_api_error(err)),
        };

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
/// Otherwise returns the input unchanged, except for known stable aliases that
/// must be discoverable before they can be run.
#[cfg(not(coverage))]
async fn resolve_screen_id(
    client: &marketsurge_client::Client,
    id_or_name: &str,
) -> Result<Option<String>, i32> {
    let response = client
        .coach_tree("marketsurge", "MSR_NAV")
        .await
        .map_err(crate::common::auth::handle_api_error)?;

    Ok(resolve_screen_id_from_response(&response, id_or_name))
}

fn resolve_screen_id_from_response(
    response: &CoachTreeResponse,
    id_or_name: &str,
) -> Option<String> {
    find_coach_screen_reference_id(response, id_or_name)
        .or_else(|| (!is_ibd_50_name(id_or_name)).then(|| id_or_name.to_string()))
}

fn find_coach_screen_reference_id(
    response: &CoachTreeResponse,
    id_or_name: &str,
) -> Option<String> {
    let normalized_target = normalized_screen_name(id_or_name);

    response
        .user
        .iter()
        .flat_map(|u| &u.screens)
        .find(|node| {
            node.name
                .as_deref()
                .is_some_and(|name| normalized_screen_name(name) == normalized_target)
        })
        .and_then(|node| node.reference_id.clone())
}

fn normalized_screen_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_ibd_50_name(name: &str) -> bool {
    normalized_screen_name(name) == "ibd50"
}

fn is_not_found_error(err: &ClientError) -> bool {
    match err {
        ClientError::GraphQL { errors } => errors.iter().any(|error| {
            error.message.contains("NOT_FOUND")
                || error
                    .extensions
                    .as_ref()
                    .is_some_and(|extensions| extensions.to_string().contains("NOT_FOUND"))
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_support::{
        optional_response_value, response_value, response_value_without_md_item,
    };
    use marketsurge_client::coach::{CoachTreeResponse, CoachTreeUser};
    use marketsurge_client::graphql::GraphQLFieldError;
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

    #[test]
    fn flatten_screen_list_handles_missing_user_screens() {
        let resp = ScreensResponse { user: None };
        let coach = make_coach_response(vec![make_coach_node("IBD 50", "ref-ibd50")]);

        let records = flatten_screen_list(&resp, Some(&coach));

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, "coach");
        assert_eq!(records[0].id.as_deref(), Some("ref-ibd50"));
    }

    #[test]
    fn resolve_screen_id_from_response_handles_stable_aliases_and_raw_ids() {
        let coach = make_coach_response(vec![make_coach_node("IBD 50", "ref-ibd50")]);

        assert_eq!(
            resolve_screen_id_from_response(&coach, "IBD50").as_deref(),
            Some("ref-ibd50")
        );
        assert_eq!(
            resolve_screen_id_from_response(&coach, "screen-custom").as_deref(),
            Some("screen-custom")
        );
    }

    #[test]
    fn resolve_screen_id_from_response_requires_ibd_50_discovery() {
        let coach = make_coach_response(vec![make_coach_node("EF-50", "ref-ef50")]);

        assert!(resolve_screen_id_from_response(&coach, "IBD 50").is_none());
        assert_eq!(
            resolve_screen_id_from_response(&coach, "EF 50").as_deref(),
            Some("ref-ef50")
        );
    }

    #[test]
    fn find_coach_screen_reference_id_matches_stable_ibd_50_aliases() {
        let coach = make_coach_response(vec![make_coach_node("IBD 50", "ref-ibd50")]);

        assert_eq!(
            find_coach_screen_reference_id(&coach, "IBD50").as_deref(),
            Some("ref-ibd50")
        );
        assert_eq!(
            find_coach_screen_reference_id(&coach, "ibd 50").as_deref(),
            Some("ref-ibd50")
        );
    }

    #[test]
    fn find_coach_screen_reference_id_keeps_similar_lists_distinct() {
        let coach = make_coach_response(vec![make_coach_node("EF-50", "ref-ef50")]);

        assert!(find_coach_screen_reference_id(&coach, "IBD 50").is_none());
        assert_eq!(
            find_coach_screen_reference_id(&coach, "EF 50").as_deref(),
            Some("ref-ef50")
        );
    }

    #[test]
    fn ibd_50_name_recognizes_only_ibd_50() {
        assert!(is_ibd_50_name("IBD 50"));
        assert!(is_ibd_50_name("ibd50"));
        assert!(!is_ibd_50_name("EF-50"));
        assert!(!is_ibd_50_name("IBD Live Watch"));
    }

    #[test]
    fn not_found_error_recognizes_graphql_not_found() {
        let err = ClientError::GraphQL {
            errors: vec![GraphQLFieldError {
                message: "screen lookup failed".to_string(),
                path: None,
                extensions: Some(serde_json::json!({"code": "NOT_FOUND"})),
            }],
        };

        assert!(is_not_found_error(&err));
    }

    #[test]
    fn not_found_error_recognizes_graphql_message_not_found() {
        let err = ClientError::GraphQL {
            errors: vec![GraphQLFieldError {
                message: "Screen NOT_FOUND".to_string(),
                path: None,
                extensions: None,
            }],
        };

        assert!(is_not_found_error(&err));
    }

    #[test]
    fn not_found_error_ignores_other_graphql_errors() {
        let err = ClientError::GraphQL {
            errors: vec![GraphQLFieldError {
                message: "permission denied".to_string(),
                path: None,
                extensions: Some(serde_json::json!({"code": "FORBIDDEN"})),
            }],
        };

        assert!(!is_not_found_error(&err));
    }

    #[test]
    fn not_found_error_ignores_non_graphql_errors() {
        let err = ClientError::BodyLimit {
            limit: 1024,
            actual: 2048,
        };

        assert!(!is_not_found_error(&err));
    }
}
