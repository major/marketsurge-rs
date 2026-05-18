//! Chart markup retrieval command.

use serde::Serialize;
use tracing::instrument;

use crate::cli::MarkupsArgs;
use crate::common::auth::handle_api_error;
use crate::common::command::run_client_command;

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

        let records: Vec<MarkupRecord> = response
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
            .collect();

        Ok(records)
    })
    .await
}
