//! Chart markup retrieval command.

use serde::Serialize;
use tracing::instrument;

use crate::cli::MarkupsArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::run_client_command;
use marketsurge_client::markups::FetchChartMarkupsResponse;

/// Flat output record for a chart markup entry.
#[derive(Debug, Clone, Serialize)]
pub struct MarkupRecord {
    /// Markup identifier.
    pub id: Option<String>,
    /// User-assigned name.
    pub name: Option<String>,
    /// Frequency (e.g. "DAILY", "WEEKLY").
    pub frequency: Option<String>,
    /// Creation timestamp.
    pub created_at: Option<String>,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Site identifier.
    pub site: Option<String>,
    /// Markup data (JSON-encoded string).
    pub data: Option<String>,
}

fn flatten_markups(response: FetchChartMarkupsResponse) -> Vec<MarkupRecord> {
    response
        .user
        .iter()
        .flat_map(|u| &u.chart_markups)
        .flat_map(|list| &list.chart_markups)
        .map(|m| MarkupRecord {
            id: m.id.clone(),
            name: m.name.clone(),
            frequency: m.frequency.clone(),
            created_at: m.created_at.clone(),
            updated_at: m.updated_at.clone(),
            site: m.site.clone(),
            data: m.data.clone(),
        })
        .collect()
}

/// Handles the markups command.
#[instrument(skip_all)]
pub async fn handle(args: &MarkupsArgs, json_table: bool) -> i32 {
    let dow_jones_key = args.dow_jones_key.clone();
    let frequency = args.frequency.clone();
    let sort_dir = args.sort_dir.clone();

    run_client_command(json_table, |client| async move {
        let response = client
            .fetch_chart_markups(
                "marketsurge",
                &dow_jones_key,
                frequency.as_deref(),
                sort_dir.as_deref(),
            )
            .await
            .map_err(handle_api_error)?;

        Ok(flatten_markups(response))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{MarkupRecord, flatten_markups};
    use marketsurge_client::markups::{
        FetchChartMarkup, FetchChartMarkupsList, FetchChartMarkupsResponse, FetchChartMarkupsUser,
    };

    fn markup(id: Option<&str>, name: Option<&str>) -> FetchChartMarkup {
        FetchChartMarkup {
            created_at: Some("2026-01-01T12:00:00Z".to_string()),
            data: Some("{\"line\":true}".to_string()),
            frequency: Some("DAILY".to_string()),
            id: id.map(ToString::to_string),
            name: name.map(ToString::to_string),
            site: Some("marketsurge".to_string()),
            updated_at: Some("2026-01-02T12:00:00Z".to_string()),
        }
    }

    fn response(user: Option<FetchChartMarkupsUser>) -> FetchChartMarkupsResponse {
        FetchChartMarkupsResponse { user }
    }

    fn user(list: Option<FetchChartMarkupsList>) -> FetchChartMarkupsUser {
        FetchChartMarkupsUser {
            chart_markups: list,
        }
    }

    fn list(chart_markups: Vec<FetchChartMarkup>) -> FetchChartMarkupsList {
        FetchChartMarkupsList {
            cursor_id: Some("cursor-1".to_string()),
            chart_markups,
        }
    }

    fn assert_record(record: &MarkupRecord, id: Option<&str>, name: Option<&str>) {
        assert_eq!(record.id.as_deref(), id);
        assert_eq!(record.name.as_deref(), name);
        assert_eq!(record.frequency.as_deref(), Some("DAILY"));
        assert_eq!(record.created_at.as_deref(), Some("2026-01-01T12:00:00Z"));
        assert_eq!(record.updated_at.as_deref(), Some("2026-01-02T12:00:00Z"));
        assert_eq!(record.site.as_deref(), Some("marketsurge"));
        assert_eq!(record.data.as_deref(), Some("{\"line\":true}"));
    }

    #[test]
    fn flatten_markups_copies_all_entries() {
        let response = response(Some(user(Some(list(vec![
            markup(Some("markup-1"), Some("First")),
            markup(Some("markup-2"), None),
        ])))));

        let records = flatten_markups(response);

        assert_eq!(records.len(), 2);
        assert_record(&records[0], Some("markup-1"), Some("First"));
        assert_record(&records[1], Some("markup-2"), None);
    }

    #[test]
    fn flatten_markups_returns_empty_when_user_missing() {
        let records = flatten_markups(response(None));

        assert!(records.is_empty());
    }

    #[test]
    fn flatten_markups_returns_empty_when_inner_list_empty() {
        let records = flatten_markups(response(Some(user(Some(list(Vec::new()))))));

        assert!(records.is_empty());
    }
}
