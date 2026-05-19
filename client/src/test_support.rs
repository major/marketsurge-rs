//! Test utilities for the marketsurge-client crate.

/// Load a fixture file from `testdata/{endpoint}/{file}` relative to the crate root.
///
/// # Panics
///
/// Panics if the file does not exist or cannot be read.
pub fn load_fixture(endpoint: &str, file: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(endpoint)
        .join(file);

    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to load fixture {}: {err}", path.display()))
}

/// Create a mock GraphQL endpoint for an operation.
#[allow(dead_code)]
pub fn mock_graphql(
    server: &mut mockito::ServerGuard,
    operation: &str,
    response_body: &str,
) -> mockito::Mock {
    server
        .mock("POST", "/gateway/graphql")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "operationName": operation
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body)
        .create()
}

/// Create a [`ClientConfig`] pointing at a mockito server's GraphQL endpoint.
pub fn test_config(server: &mockito::ServerGuard) -> crate::client::ClientConfig {
    let graphql_url = url::Url::parse(&format!("{}/gateway/graphql", server.url()))
        .expect("mock server URL should be valid");
    crate::client::ClientConfig::default().with_graphql_url(graphql_url)
}

/// Create a mock GraphQL endpoint and client for testing.
///
/// Returns the mock server guard (which must be kept alive), a configured
/// [`Client`], and the [`mockito::Mock`] for assertion.
///
/// # Panics
///
/// Panics if the fixture file does not exist or the client cannot be built.
pub async fn mock_test(
    operation: &str,
) -> (mockito::ServerGuard, crate::client::Client, mockito::Mock) {
    mock_test_with_fixture(operation, operation).await
}

/// Like [`mock_test`], but allows a fixture endpoint name that differs from
/// the GraphQL operation name.
///
/// # Panics
///
/// Panics if the fixture file does not exist or the client cannot be built.
pub async fn mock_test_with_fixture(
    fixture_endpoint: &str,
    operation: &str,
) -> (mockito::ServerGuard, crate::client::Client, mockito::Mock) {
    let mut server = mockito::Server::new_async().await;
    let response_body = load_fixture(fixture_endpoint, "response.json");
    let mock = mock_graphql(&mut server, operation, &response_body);
    let client = crate::client::Client::new(test_config(&server)).expect("client should build");
    (server, client, mock)
}

/// Creates a live client authenticated from local Firefox cookies.
///
/// # Panics
///
/// Panics if Firefox cookies are missing or JWT exchange fails.
#[cfg(not(coverage))]
pub async fn live_client() -> crate::client::Client {
    crate::client::Client::from_browser()
        .await
        .expect("expected live browser-authenticated MarketSurge client")
}

#[cfg(test)]
mod tests {
    use super::load_fixture;

    #[cfg(not(coverage))]
    use super::live_client;

    #[test]
    fn loads_fixture_content() {
        let fixture = load_fixture("ChartMarketData", "response.json");
        assert!(!fixture.trim().is_empty());
    }

    #[cfg(not(coverage))]
    #[tokio::test]
    #[ignore]
    async fn live_client_works_with_browser_session() {
        let _client = live_client().await;
    }
}
