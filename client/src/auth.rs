//! Browser-cookie authentication for MarketSurge.

use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{COOKIE, HeaderName, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use serde::Deserialize;
use tracing::instrument;
use url::Url;

use crate::error::{ClientError, Result};

const JWT_EXCHANGE_PATH: &str = "/client";
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:149.0) Gecko/20100101 Firefox/149.0";
const REFERER_URL: &str = "https://marketsurge.investors.com/";
const ORIGIN_URL: &str = "https://marketsurge.investors.com";
const MARKET_SURGE_ORIGINAL_HOST: &str = "marketsurge.investors.com";
const MARKET_SURGE_ORIGINAL_URL: &str = "/mstool";

const X_ENCRYPTED_DOCUMENT_KEY: HeaderName = HeaderName::from_static("x-encrypted-document-key");
const X_ORIGINAL_HOST: HeaderName = HeaderName::from_static("x-original-host");
const X_ORIGINAL_REFERRER: HeaderName = HeaderName::from_static("x-original-referrer");
const X_ORIGINAL_URL: HeaderName = HeaderName::from_static("x-original-url");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientInfoResponse {
    is_logged_in: bool,
    jwt: String,
}

/// Exchanges authenticated investors.com browser cookies for a MarketSurge JWT.
///
/// # Errors
///
/// Returns [`ClientError::Status`] for non-success HTTP responses or an
/// unauthenticated exchange response. Returns wrapped reqwest/serde errors for
/// transport and JSON failures.
#[instrument(skip_all)]
pub async fn exchange_jwt(
    http: &reqwest::Client,
    investors_base_url: &Url,
    cookie_jar: &Jar,
) -> Result<String> {
    let mut url = investors_base_url.clone();
    url.set_path(JWT_EXCHANGE_PATH);
    url.set_query(None);
    url.set_fragment(None);
    let mut request = http
        .get(url.clone())
        .header(USER_AGENT, DEFAULT_USER_AGENT)
        .header(X_ENCRYPTED_DOCUMENT_KEY, HeaderValue::from_static(""))
        .header(X_ORIGINAL_HOST, MARKET_SURGE_ORIGINAL_HOST)
        .header(X_ORIGINAL_REFERRER, HeaderValue::from_static(""))
        .header(X_ORIGINAL_URL, MARKET_SURGE_ORIGINAL_URL)
        .header(REFERER, REFERER_URL)
        .header(ORIGIN, ORIGIN_URL);

    if let Some(cookies) = cookie_jar.cookies(&url) {
        request = request.header(COOKIE, cookies);
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let bytes = response.bytes().await?;
        return Err(ClientError::Status {
            status: status.as_u16(),
            body: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }

    let info = response.json::<ClientInfoResponse>().await?;
    if !info.is_logged_in {
        return Err(ClientError::Status {
            status: 401,
            body: "not logged in: check that you are signed into MarketSurge in the browser"
                .to_string(),
        });
    }
    if info.jwt.is_empty() {
        return Err(ClientError::Status {
            status: 401,
            body: "JWT not found in exchange response".to_string(),
        });
    }

    Ok(info.jwt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_get_response;

    fn test_cookie_jar(base_url: &Url) -> Jar {
        let jar = Jar::default();
        jar.add_cookie_str("ibd-session=session-value", base_url);
        jar
    }

    #[tokio::test]
    async fn test_jwt_exchange_success() {
        let mut server = mockito::Server::new_async().await;
        let base_url = Url::parse(&server.url()).expect("mock server URL should parse");
        let mock = server
            .mock("GET", JWT_EXCHANGE_PATH)
            .match_header("user-agent", DEFAULT_USER_AGENT)
            .match_header("x-encrypted-document-key", "")
            .match_header("x-original-host", MARKET_SURGE_ORIGINAL_HOST)
            .match_header("x-original-referrer", "")
            .match_header("x-original-url", MARKET_SURGE_ORIGINAL_URL)
            .match_header("referer", REFERER_URL)
            .match_header("origin", ORIGIN_URL)
            .match_header("cookie", "ibd-session=session-value")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"isLoggedIn":true,"jwt":"jwt-token","given_name":"Major","family_name":"Hayden"}"#)
            .create_async()
            .await;
        let http = reqwest::Client::new();
        let jar = test_cookie_jar(&base_url);

        let jwt = exchange_jwt(&http, &base_url, &jar)
            .await
            .expect("JWT exchange should succeed");

        assert_eq!(jwt, "jwt-token");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_jwt_exchange_401() {
        let mut server = mockito::Server::new_async().await;
        let base_url = Url::parse(&server.url()).expect("mock server URL should parse");
        mock_get_response(&mut server, JWT_EXCHANGE_PATH, 401, "unauthorized");
        let http = reqwest::Client::new();
        let jar = test_cookie_jar(&base_url);

        let error = exchange_jwt(&http, &base_url, &jar)
            .await
            .expect_err("JWT exchange should fail");

        match error {
            ClientError::Status { status, body } => {
                assert_eq!(status, 401);
                assert_eq!(body, "unauthorized");
            }
            other => panic!("expected Status error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_jwt_exchange_rejects_logged_out_response() {
        let mut server = mockito::Server::new_async().await;
        let base_url = Url::parse(&server.url()).expect("mock server URL should parse");
        mock_get_response(
            &mut server,
            JWT_EXCHANGE_PATH,
            200,
            r#"{"isLoggedIn":false,"jwt":"ignored"}"#,
        );
        let http = reqwest::Client::new();
        let jar = test_cookie_jar(&base_url);

        let error = exchange_jwt(&http, &base_url, &jar)
            .await
            .expect_err("logged-out exchange should fail");

        match error {
            ClientError::Status { status, body } => {
                assert_eq!(status, 401);
                assert!(body.contains("not logged in"));
            }
            other => panic!("expected Status error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_jwt_exchange_rejects_empty_jwt() {
        let mut server = mockito::Server::new_async().await;
        let base_url = Url::parse(&server.url()).expect("mock server URL should parse");
        mock_get_response(
            &mut server,
            JWT_EXCHANGE_PATH,
            200,
            r#"{"isLoggedIn":true,"jwt":""}"#,
        );
        let http = reqwest::Client::new();
        let jar = test_cookie_jar(&base_url);

        let error = exchange_jwt(&http, &base_url, &jar)
            .await
            .expect_err("empty JWT exchange should fail");

        match error {
            ClientError::Status { status, body } => {
                assert_eq!(status, 401);
                assert_eq!(body, "JWT not found in exchange response");
            }
            other => panic!("expected Status error, got {other:?}"),
        }
    }
}
