//! Chart markup endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_FETCH_CHART_MARKUPS: &str = r#"query FetchChartMarkups($site: Site!, $dowJonesKey: String, $frequency: ChartMarkupFrequencyInput, $dateStart: String, $dateEnd: String, $cursorId: String, $limit: Int, $sortDir: SortDirInput) {
  user {
    chartMarkups(
      site: $site
      dowJonesKey: $dowJonesKey
      frequency: $frequency
      dateStart: $dateStart
      dateEnd: $dateEnd
      cursorId: $cursorId
      limit: $limit
      sortDir: $sortDir
    ) {
      cursorId
      chartMarkups {
        createdAt
        data
        frequency
        id
        name
        site
        updatedAt
      }
    }
  }
}"#;

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchChartMarkupsVariables {
    site: String,
    dow_jones_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i32>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `FetchChartMarkups` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchChartMarkupsResponse {
    /// User-scoped markup data.
    pub user: Option<FetchChartMarkupsUser>,
}

/// User-scoped chart markups wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchChartMarkupsUser {
    /// Paginated chart markups list.
    pub chart_markups: Option<FetchChartMarkupsList>,
}

/// Paginated list of chart markups.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchChartMarkupsList {
    /// Pagination cursor.
    pub cursor_id: Option<String>,
    /// Markup entries.
    #[serde(default)]
    pub chart_markups: Vec<FetchChartMarkup>,
}

/// Single user-saved chart markup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchChartMarkup {
    /// Creation timestamp.
    pub created_at: Option<String>,
    /// Markup data (JSON-encoded string).
    pub data: Option<String>,
    /// Frequency (e.g. "DAILY", "WEEKLY").
    pub frequency: Option<String>,
    /// Markup identifier.
    pub id: Option<String>,
    /// User-assigned name.
    pub name: Option<String>,
    /// Site identifier.
    pub site: Option<String>,
    /// Last update timestamp.
    pub updated_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Client method
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches user-saved chart markups.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn fetch_chart_markups(
        &self,
        site: &str,
        dow_jones_key: &str,
        frequency: Option<&str>,
        sort_dir: Option<&str>,
    ) -> crate::error::Result<FetchChartMarkupsResponse> {
        let variables = FetchChartMarkupsVariables {
            site: site.to_string(),
            dow_jones_key: dow_jones_key.to_string(),
            frequency: frequency.map(ToString::to_string),
            sort_dir: sort_dir.map(ToString::to_string),
            date_start: None,
            date_end: None,
            cursor_id: None,
            limit: None,
        };

        let request = GraphQLRequest {
            operation_name: "FetchChartMarkups".to_string(),
            variables,
            query: QUERY_FETCH_CHART_MARKUPS.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn fetch_chart_markups_parses_response() {
        let (_server, client, mock) = mock_test("FetchChartMarkups").await;

        let resp = client
            .fetch_chart_markups("marketsurge", "13-5320", None, None)
            .await
            .expect("fetch_chart_markups should succeed");

        let user = resp.user.as_ref().expect("user");
        let list = user.chart_markups.as_ref().expect("chart_markups");
        assert_eq!(list.cursor_id.as_deref(), Some("cursor-abc123"));
        assert_eq!(list.chart_markups.len(), 2);

        let first = &list.chart_markups[0];
        assert_eq!(first.id.as_deref(), Some("markup-001"));
        assert_eq!(first.name.as_deref(), Some("My Trend Lines"));
        assert_eq!(first.frequency.as_deref(), Some("DAILY"));
        assert!(first.data.is_some());

        let second = &list.chart_markups[1];
        assert_eq!(second.id.as_deref(), Some("markup-002"));
        assert!(second.name.is_none());
        assert!(second.updated_at.is_none());

        mock.assert();
    }

    #[tokio::test]
    #[ignore]
    async fn integration_fetch_chart_markups() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .fetch_chart_markups("marketsurge", "13-5320", None, None)
            .await
            .expect("live fetch_chart_markups should succeed");

        assert!(resp.user.is_some());
    }
}
