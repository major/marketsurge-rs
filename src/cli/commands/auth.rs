//! Auth status command: verify browser cookie and JWT readiness without
//! fetching market data.

use serde::Serialize;

use crate::cli::common::exit::ExitCode;
#[cfg(not(coverage))]
use crate::cli::output::{finish_output, print_json};

/// JSON payload returned by `auth status` on success.
#[derive(Debug, Serialize, PartialEq)]
pub struct AuthStatus {
    /// Whether usable MarketSurge auth material is available.
    pub authenticated: bool,
    /// Source of browser cookies (currently always "firefox").
    pub source: String,
    /// Number of investors.com cookies found in the browser profile.
    pub cookie_count: usize,
}

/// Run the auth status check: extract browser cookies, exchange for a JWT,
/// and report readiness without fetching market data.
#[cfg(not(coverage))]
pub async fn handle(fields: &[String]) -> i32 {
    // Step 1: Extract browser cookies (local, no network).
    let cookies = match crate::browser_auth::extract_cookies() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("auth error: {err}");
            return ExitCode::AuthError.code();
        }
    };

    let cookie_count = cookies.len();

    // Step 2: Build cookie jar (local).
    let cookie_jar = match crate::browser_auth::build_cookie_jar(&cookies) {
        Ok(jar) => jar,
        Err(err) => {
            eprintln!("auth error: {err}");
            return ExitCode::AuthError.code();
        }
    };

    // Step 3: Exchange cookies for JWT (network, but not market data).
    let config = crate::ClientConfig::default();
    let http = match reqwest::Client::builder().timeout(config.timeout).build() {
        Ok(client) => client,
        Err(err) => {
            eprintln!("client error: {err}");
            return ExitCode::InternalError.code();
        }
    };

    match crate::auth::exchange_jwt(&http, &config.investors_base_url, &cookie_jar).await {
        Ok(_) => {
            let status = AuthStatus {
                authenticated: true,
                source: "firefox".to_string(),
                cookie_count,
            };
            finish_output(print_json(&status, fields))
        }
        Err(err) => {
            if err.is_auth_error() {
                eprintln!("auth error: {err}");
                ExitCode::AuthError.code()
            } else {
                eprintln!("API error: {err}");
                ExitCode::ApiError.code()
            }
        }
    }
}

/// Coverage stub: returns success so instrumentation does not hit live
/// browser I/O.
#[cfg(coverage)]
pub async fn handle(_fields: &[String]) -> i32 {
    ExitCode::Success.code()
}

#[cfg(test)]
mod tests {
    use super::AuthStatus;
    use crate::cli::common::exit::ExitCode;

    #[test]
    fn auth_status_serializes_to_compact_json() {
        let status = AuthStatus {
            authenticated: true,
            source: "firefox".to_string(),
            cookie_count: 3,
        };

        let json = serde_json::to_string(&status).expect("AuthStatus should serialize");

        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized AuthStatus should be valid JSON");
        assert_eq!(parsed["authenticated"], true);
        assert_eq!(parsed["source"], "firefox");
        assert_eq!(parsed["cookie_count"], 3);
    }

    #[test]
    fn auth_status_unauthenticated_serializes() {
        let status = AuthStatus {
            authenticated: false,
            source: "firefox".to_string(),
            cookie_count: 0,
        };

        let json = serde_json::to_string(&status).expect("AuthStatus should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized AuthStatus should be valid JSON");
        assert_eq!(parsed["authenticated"], false);
        assert_eq!(parsed["cookie_count"], 0);
    }

    #[test]
    fn auth_status_no_secrets_in_fields() {
        let status = AuthStatus {
            authenticated: true,
            source: "firefox".to_string(),
            cookie_count: 2,
        };

        let json = serde_json::to_string(&status).expect("AuthStatus should serialize");

        // No cookie values, tokens, or auth headers in output.
        assert!(!json.contains("ibd-session"));
        assert!(!json.contains("Bearer"));
        assert!(!json.contains("jwt"));
    }

    #[test]
    fn exit_code_values_are_stable() {
        // Verify the exit codes used by auth status are documented values.
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::InternalError.code(), 1);
        assert_eq!(ExitCode::AuthError.code(), 4);
    }
}
