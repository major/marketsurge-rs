//! Browser cookie authentication for MarketSurge.

use crate::{Client, ClientError};

/// Build a MarketSurge client from browser cookies and a JWT exchange.
#[cfg(not(test))]
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

/// Build a test client without reading browser cookies.
#[cfg(test)]
pub async fn make_client() -> Result<Client, i32> {
    Client::new(crate::ClientConfig::default()).map_err(handle_api_error)
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

#[cfg(test)]
mod tests {
    use super::handle_api_error;
    use crate::ClientError;

    fn status_error(status: u16) -> ClientError {
        ClientError::Status {
            status,
            body: String::new(),
        }
    }

    #[test]
    fn test_handle_api_error_auth_returns_2() {
        assert_eq!(handle_api_error(status_error(401)), 2);
    }

    #[test]
    fn test_handle_api_error_other_returns_1() {
        assert_eq!(handle_api_error(status_error(500)), 1);
    }
}
