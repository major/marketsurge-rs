//! Navigation tree endpoints.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_NAV_TREE: &str = r#"query NavTree($site: Site!, $treeType: NavTreeTypeInput!) {
  user {
    navTree(site: $site, treeType: $treeType) {
      ... on NavTreeFolder {
        id
        name
        parentId
        type
        children {
          ... on NavTreeFolder { id name type }
          ... on NavTreeLeaf { id name type }
        }
        contentType
        treeType
      }
      ... on NavTreeLeaf {
        id
        name
        parentId
        type
        url
        treeType
        referenceId
      }
    }
  }
}"#;

// ---------------------------------------------------------------------------
// Wire variable types (serialization only)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NavTreeVariables {
    site: String,
    tree_type: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `NavTree` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavTreeResponse {
    /// User-scoped navigation tree data.
    pub user: Option<NavTreeUser>,
}

/// User-scoped navigation tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavTreeUser {
    /// Navigation tree nodes.
    #[serde(default)]
    pub nav_tree: Vec<NavTreeNode>,
}

/// A node in the navigation tree.
pub type NavTreeNode = crate::types::TreeNode;

/// A child node summary within a navigation tree folder.
pub type NavTreeChildNode = crate::types::TreeChildNode;

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches the full navigation tree for the authenticated user.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn nav_tree(
        &self,
        site: &str,
        tree_type: &str,
    ) -> crate::error::Result<NavTreeResponse> {
        let variables = NavTreeVariables {
            site: site.to_string(),
            tree_type: tree_type.to_string(),
        };

        let request = GraphQLRequest {
            operation_name: "NavTree".to_string(),
            variables,
            query: QUERY_NAV_TREE.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn nav_tree_parses_response() {
        let (_server, client, mock) = mock_test("NavTree").await;

        let resp = client
            .nav_tree("marketsurge", "MSR_NAV")
            .await
            .expect("nav_tree should succeed");

        let user = resp.user.as_ref().expect("user");
        assert_eq!(user.nav_tree.len(), 3);

        // Folder node
        let folder = &user.nav_tree[0];
        assert_eq!(folder.id.as_deref(), Some("folder-reports"));
        assert_eq!(folder.name.as_deref(), Some("My Reports"));
        assert_eq!(folder.node_type.as_deref(), Some("SYSTEM_FOLDER"));
        assert_eq!(folder.content_type.as_deref(), Some("REPORTS"));
        assert_eq!(folder.children.len(), 2);
        assert_eq!(folder.children[0].id.as_deref(), Some("report-120"));
        assert_eq!(
            folder.children[0].name.as_deref(),
            Some("Minervini Trend - 5 Months")
        );

        // Leaf node with JSON reference_id
        let report = &user.nav_tree[1];
        assert_eq!(report.id.as_deref(), Some("report-120"));
        assert_eq!(report.node_type.as_deref(), Some("REPORTS_SCREEN"));
        assert_eq!(report.url.as_deref(), Some("/report/minervini-5m"));
        assert_eq!(
            report.reference_id.as_deref(),
            Some(r#"{"originalId":120,"isCoachAccount":false}"#)
        );

        // Screen leaf
        let screen = &user.nav_tree[2];
        assert_eq!(screen.id.as_deref(), Some("leaf-screen-1"));
        assert_eq!(screen.name.as_deref(), Some("My Growth Screen"));
        assert_eq!(screen.node_type.as_deref(), Some("STOCK_SCREEN"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_nav_tree() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .nav_tree("marketsurge", "MSR_NAV")
            .await
            .expect("live nav_tree should succeed");

        assert!(resp.user.is_some());
    }
}
