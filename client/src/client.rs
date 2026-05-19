//! HTTP client for MarketSurge GraphQL requests.

use std::time::Duration;

use reqwest::header::{
    AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER, USER_AGENT,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::instrument;
use url::Url;

use crate::error::{ClientError, Result};
use crate::graphql::{GraphQLRequest, GraphQLResponse};

const DEFAULT_GRAPHQL_URL: &str = "https://shared-data.dowjones.io/gateway/graphql";
const DEFAULT_INVESTORS_BASE_URL: &str = "https://www.investors.com";
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:149.0) Gecko/20100101 Firefox/149.0";
const DEFAULT_BODY_LIMIT: usize = 10 * 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 10;

const APOLLO_CLIENT_NAME: HeaderName = HeaderName::from_static("apollographql-client-name");
const DYLAN_ENTITLEMENT_TOKEN: HeaderName = HeaderName::from_static("dylan-entitlement-token");
const DYLAN_TOKEN: &str = "x4ckyhshg90pdq6bwf6n1voijs7r3fdk";
const MARKET_SURGE_CLIENT_NAME: &str = "marketsurge";
const REFERER_URL: &str = "https://marketsurge.investors.com/";
const ORIGIN_URL: &str = "https://marketsurge.investors.com";
const AUTHORIZATION_BEARER_PREFIX: &str = "Bearer ";

/// HTTP client configuration.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// GraphQL gateway URL.
    pub graphql_url: Url,
    /// Base URL for investors.com browser authentication flows.
    pub investors_base_url: Url,
    /// Browser user-agent sent with requests.
    pub user_agent: String,
    /// Maximum response body size read into memory.
    pub body_limit: usize,
    /// Per-request HTTP timeout.
    pub timeout: Duration,
    /// Optional JWT used for GraphQL Authorization.
    pub jwt: Option<String>,
    /// Additional default headers, applied after built-in headers.
    pub extra_headers: HeaderMap,
}

impl ClientConfig {
    /// Sets the GraphQL gateway URL.
    #[must_use]
    pub fn with_graphql_url(mut self, graphql_url: Url) -> Self {
        self.graphql_url = graphql_url;
        self
    }

    /// Sets the investors.com base URL.
    #[must_use]
    pub fn with_investors_base_url(mut self, investors_base_url: Url) -> Self {
        self.investors_base_url = investors_base_url;
        self
    }

    /// Sets the browser user-agent header value.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Sets the maximum response body size.
    #[must_use]
    pub fn with_body_limit(mut self, body_limit: usize) -> Self {
        self.body_limit = body_limit;
        self
    }

    /// Sets the per-request HTTP timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the JWT used for GraphQL Authorization.
    #[must_use]
    pub fn with_jwt(mut self, jwt: impl Into<String>) -> Self {
        self.jwt = Some(jwt.into());
        self
    }

    /// Adds or replaces an extra default header.
    #[must_use]
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.extra_headers.insert(name, value);
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            graphql_url: parse_static_url(DEFAULT_GRAPHQL_URL),
            investors_base_url: parse_static_url(DEFAULT_INVESTORS_BASE_URL),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            body_limit: DEFAULT_BODY_LIMIT,
            timeout: DEFAULT_TIMEOUT,
            jwt: None,
            extra_headers: HeaderMap::new(),
        }
    }
}

/// MarketSurge GraphQL HTTP client.
#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
    config: ClientConfig,
}

impl Client {
    /// Creates a GraphQL client from explicit configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Http`] if the reqwest client cannot be built.
    pub fn new(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .default_headers(default_headers(&config))
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .cookie_store(true)
            .build()?;

        Ok(Self { http, config })
    }

    /// Creates a client authenticated from local Firefox investors.com cookies.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Status`] if browser cookies are missing or the JWT
    /// exchange fails. Returns [`ClientError::Http`] if the reqwest client cannot
    /// be built or the exchange request fails.
    #[instrument(skip_all)]
    pub async fn from_browser() -> Result<Self> {
        let config = ClientConfig::default();
        let cookies = crate::browser_auth::extract_cookies()?;
        let cookie_jar = crate::browser_auth::build_cookie_jar(&cookies)?;
        let http = reqwest::Client::builder().timeout(config.timeout).build()?;
        let jwt = crate::auth::exchange_jwt(&http, &config.investors_base_url, &cookie_jar).await?;

        Self::new(config.with_jwt(jwt))
    }

    /// Sends a GraphQL POST request and returns the response data payload.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Status`] for non-success HTTP responses,
    /// [`ClientError::BodyLimit`] when the response exceeds the configured body
    /// limit, [`ClientError::GraphQL`] when the GraphQL response contains only
    /// errors, or wrapped reqwest/serde errors for transport and JSON failures.
    #[instrument(skip_all)]
    pub async fn graphql_post<V, T>(&self, request: &GraphQLRequest<V>) -> Result<T>
    where
        V: Serialize,
        T: DeserializeOwned,
    {
        let body = serde_json::to_string(request)?;
        let response = self
            .http
            .post(self.config.graphql_url.clone())
            .body(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let bytes = response.bytes().await?;
            return Err(ClientError::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }

        let bytes = response.bytes().await?;
        if bytes.len() > self.config.body_limit {
            return Err(ClientError::BodyLimit {
                limit: self.config.body_limit,
                actual: bytes.len(),
            });
        }

        let response = serde_json::from_slice::<GraphQLResponse<T>>(&bytes)?;
        if let Some(data) = response.data {
            return Ok(data);
        }

        if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
            return Err(ClientError::GraphQL { errors });
        }

        Ok(serde_json::from_value(serde_json::Value::Null)?)
    }

    /// Sends a typed GraphQL operation with the shared request envelope.
    pub(crate) async fn graphql_operation<V, T>(
        &self,
        operation_name: &str,
        variables: V,
        query: impl Into<String>,
    ) -> Result<T>
    where
        V: Serialize,
        T: DeserializeOwned,
    {
        let request = GraphQLRequest::new(operation_name, variables, query);
        self.graphql_post(&request).await
    }
}

fn default_headers(config: &ClientConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        APOLLO_CLIENT_NAME,
        HeaderValue::from_static(MARKET_SURGE_CLIENT_NAME),
    );
    headers.insert(
        DYLAN_ENTITLEMENT_TOKEN,
        HeaderValue::from_static(DYLAN_TOKEN),
    );
    headers.insert(REFERER, HeaderValue::from_static(REFERER_URL));
    headers.insert(ORIGIN, HeaderValue::from_static(ORIGIN_URL));
    insert_header_value(&mut headers, USER_AGENT, &config.user_agent);
    if let Some(jwt) = config.jwt.as_deref() {
        insert_header_value(
            &mut headers,
            AUTHORIZATION,
            &format!("{AUTHORIZATION_BEARER_PREFIX}{jwt}"),
        );
    }
    headers.extend(config.extra_headers.clone());
    headers
}

fn insert_header_value(headers: &mut HeaderMap, name: HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

fn parse_static_url(raw: &str) -> Url {
    match Url::parse(raw) {
        Ok(url) => url,
        Err(error) => panic!("invalid static URL {raw}: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{mock_graphql_response, test_client, test_config};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        value: u32,
    }

    fn request() -> GraphQLRequest<serde_json::Value> {
        GraphQLRequest {
            operation_name: "TestOperation".to_string(),
            variables: serde_json::json!({}),
            query: "query TestOperation { value }".to_string(),
        }
    }

    #[tokio::test]
    async fn test_successful_post() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/gateway/graphql")
            .match_header("content-type", "application/json")
            .match_header("apollographql-client-name", MARKET_SURGE_CLIENT_NAME)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"value":42},"errors":null}"#)
            .create_async()
            .await;
        let client = test_client(&server);

        let data: TestData = client
            .graphql_post(&request())
            .await
            .expect("GraphQL request should succeed");

        assert_eq!(data, TestData { value: 42 });
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_non_2xx_returns_status_error() {
        let mut server = mockito::Server::new_async().await;
        mock_graphql_response(&mut server, 401, "unauthorized");
        let client = test_client(&server);

        let error = client
            .graphql_post::<_, TestData>(&request())
            .await
            .expect_err("GraphQL request should fail");

        match error {
            ClientError::Status { status, body } => {
                assert_eq!(status, 401);
                assert_eq!(body, "unauthorized");
            }
            other => panic!("expected Status error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_body_limit_exceeded() {
        let mut server = mockito::Server::new_async().await;
        mock_graphql_response(&mut server, 200, r#"{"data":{"value":42},"errors":null}"#);
        let config = test_config(&server).with_body_limit(5);
        let client = Client::new(config).expect("client should build");

        let error = client
            .graphql_post::<_, TestData>(&request())
            .await
            .expect_err("GraphQL request should fail");

        match error {
            ClientError::BodyLimit { limit, .. } => assert_eq!(limit, 5),
            other => panic!("expected BodyLimit error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_graphql_errors_returns_graphql_error() {
        let mut server = mockito::Server::new_async().await;
        mock_graphql_response(
            &mut server,
            200,
            r#"{"data":null,"errors":[{"message":"not found","path":null,"extensions":null}]}"#,
        );
        let client = test_client(&server);

        let error = client
            .graphql_post::<_, TestData>(&request())
            .await
            .expect_err("GraphQL request should fail");

        match error {
            ClientError::GraphQL { errors } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].message, "not found");
            }
            other => panic!("expected GraphQL error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_json_decode_error() {
        let mut server = mockito::Server::new_async().await;
        mock_graphql_response(&mut server, 200, "not json");
        let client = test_client(&server);

        let error = client
            .graphql_post::<_, TestData>(&request())
            .await
            .expect_err("GraphQL request should fail");

        assert!(matches!(error, ClientError::Json(_)));
    }

    #[tokio::test]
    async fn test_default_config_values() {
        let config = ClientConfig::default();

        assert_eq!(
            config.graphql_url,
            Url::parse(DEFAULT_GRAPHQL_URL).expect("default GraphQL URL should parse")
        );
        assert_eq!(
            config.investors_base_url,
            Url::parse(DEFAULT_INVESTORS_BASE_URL).expect("default investors URL should parse")
        );
        assert_eq!(config.user_agent, DEFAULT_USER_AGENT);
        assert_eq!(config.body_limit, DEFAULT_BODY_LIMIT);
        assert_eq!(config.timeout, DEFAULT_TIMEOUT);
        assert_eq!(config.jwt, None);
        assert!(config.extra_headers.is_empty());
    }

    #[test]
    fn config_builders_override_defaults() {
        let graphql_url = Url::parse("https://example.test/graphql").unwrap();
        let investors_base_url = Url::parse("https://investors.example.test").unwrap();
        let header_name = HeaderName::from_static("x-test-header");
        let header_value = HeaderValue::from_static("header-value");

        let config = ClientConfig::default()
            .with_graphql_url(graphql_url.clone())
            .with_investors_base_url(investors_base_url.clone())
            .with_user_agent("test-agent")
            .with_body_limit(42)
            .with_timeout(Duration::from_secs(7))
            .with_jwt("jwt-token")
            .with_header(header_name.clone(), header_value.clone());

        assert_eq!(config.graphql_url, graphql_url);
        assert_eq!(config.investors_base_url, investors_base_url);
        assert_eq!(config.user_agent, "test-agent");
        assert_eq!(config.body_limit, 42);
        assert_eq!(config.timeout, Duration::from_secs(7));
        assert_eq!(config.jwt.as_deref(), Some("jwt-token"));
        assert_eq!(config.extra_headers.get(header_name), Some(&header_value));
    }

    #[test]
    fn default_headers_include_jwt_and_extra_headers() {
        let config = ClientConfig::default().with_jwt("jwt-token").with_header(
            HeaderName::from_static("x-extra"),
            HeaderValue::from_static("extra"),
        );

        let headers = default_headers(&config);

        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer jwt-token");
        assert_eq!(headers.get("x-extra").unwrap(), "extra");
        assert_eq!(headers.get(USER_AGENT).unwrap(), DEFAULT_USER_AGENT);
    }

    #[test]
    fn insert_header_value_drops_invalid_values() {
        let mut headers = HeaderMap::new();

        insert_header_value(&mut headers, USER_AGENT, "bad\nvalue");

        assert!(!headers.contains_key(USER_AGENT));
    }

    #[tokio::test]
    async fn test_null_data_without_errors_deserializes_null() {
        let mut server = mockito::Server::new_async().await;
        mock_graphql_response(&mut server, 200, r#"{"data":null,"errors":null}"#);
        let client = test_client(&server);

        let data: Option<TestData> = client
            .graphql_post(&request())
            .await
            .expect("null GraphQL data should deserialize");

        assert_eq!(data, None);
    }

    #[test]
    #[should_panic(expected = "invalid static URL")]
    fn parse_static_url_panics_for_invalid_constants() {
        parse_static_url("not a url");
    }
}
