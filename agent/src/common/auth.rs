//! Browser cookie authentication for MarketSurge.

use marketsurge_client::{Client, ClientError};

/// Build a MarketSurge client from browser cookies and a JWT exchange.
pub async fn make_client() -> Result<Client, i32> {
    match Client::from_browser().await {
        Ok(client) => Ok(client),
        Err(err) => {
            if err.is_auth_error() {
                eprintln!("auth error: {err}");
                Err(2)
            } else {
                eprintln!("client error: {err}");
                Err(1)
            }
        }
    }
}

/// Convert API errors into CLI exit codes and messages.
pub fn handle_api_error(err: ClientError) -> i32 {
    if err.is_auth_error() {
        eprintln!("auth error: {err}");
        2
    } else {
        eprintln!("API error: {err}");
        1
    }
}
