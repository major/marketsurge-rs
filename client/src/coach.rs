//! Coach tree navigation endpoints (watchlists and screens).

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::graphql::GraphQLRequest;

// ---------------------------------------------------------------------------
// GraphQL query
// ---------------------------------------------------------------------------

const QUERY_COACH_TREE: &str = r#"query CoachTree($site: Site!, $treeType: NavTreeTypeInput!) {
  user {
    watchlists: coachTree(
      coachTreeType: WATCHLIST
      site: $site
      treeType: $treeType
    ) {
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
    screens: coachTree(
      coachTreeType: SCREEN
      site: $site
      treeType: $treeType
    ) {
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
struct CoachTreeVariables {
    site: String,
    tree_type: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Top-level response from the `CoachTree` query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachTreeResponse {
    /// User-scoped coach tree data.
    pub user: Option<CoachTreeUser>,
}

/// User-scoped coach tree containing watchlists and screens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachTreeUser {
    /// Watchlist tree nodes.
    #[serde(default)]
    pub watchlists: Vec<CoachTreeNode>,
    /// Screen tree nodes.
    #[serde(default)]
    pub screens: Vec<CoachTreeNode>,
}

/// A node in the coach tree.
pub type CoachTreeNode = crate::types::TreeNode;

/// A child node summary within a coach tree folder.
pub type CoachTreeChildNode = crate::types::TreeChildNode;

// ---------------------------------------------------------------------------
// Client methods
// ---------------------------------------------------------------------------

impl Client {
    /// Fetches the coach tree for watchlists and screens.
    ///
    /// # Errors
    ///
    /// Returns an error if the GraphQL request fails or the response
    /// cannot be deserialized.
    pub async fn coach_tree(
        &self,
        site: &str,
        tree_type: &str,
    ) -> crate::error::Result<CoachTreeResponse> {
        let variables = CoachTreeVariables {
            site: site.to_string(),
            tree_type: tree_type.to_string(),
        };

        let request = GraphQLRequest {
            operation_name: "CoachTree".to_string(),
            variables,
            query: QUERY_COACH_TREE.to_string(),
        };

        self.graphql_post(&request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::mock_test;

    #[tokio::test]
    async fn coach_tree_parses_response() {
        let (_server, client, mock) = mock_test("CoachTree").await;

        let resp = client
            .coach_tree("marketsurge", "MSR_NAV")
            .await
            .expect("coach_tree should succeed");

        let user = resp.user.as_ref().expect("user");

        // Watchlists
        assert_eq!(user.watchlists.len(), 2);
        let folder = &user.watchlists[0];
        assert_eq!(folder.id.as_deref(), Some("folder-1"));
        assert_eq!(folder.name.as_deref(), Some("My Watchlists"));
        assert_eq!(folder.node_type.as_deref(), Some("FOLDER"));
        assert_eq!(folder.content_type.as_deref(), Some("WATCHLIST"));
        assert_eq!(folder.children.len(), 1);
        assert_eq!(folder.children[0].id.as_deref(), Some("leaf-1"));
        assert_eq!(folder.children[0].name.as_deref(), Some("Growth Stocks"));

        let leaf = &user.watchlists[1];
        assert_eq!(leaf.id.as_deref(), Some("leaf-2"));
        assert_eq!(leaf.url.as_deref(), Some("/watchlist/tech-leaders"));
        assert_eq!(leaf.reference_id.as_deref(), Some("12345"));

        // Screens
        assert_eq!(user.screens.len(), 1);
        assert_eq!(user.screens[0].id.as_deref(), Some("screen-leaf-1"));
        assert_eq!(user.screens[0].name.as_deref(), Some("IBD 50"));
        assert_eq!(user.screens[0].url.as_deref(), Some("/screen/ibd50"));

        mock.assert();
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn integration_coach_tree() {
        let client = crate::test_support::live_client().await;
        let resp = client
            .coach_tree("marketsurge", "MSR_NAV")
            .await
            .expect("live coach_tree should succeed");

        assert!(resp.user.is_some());
    }
}
