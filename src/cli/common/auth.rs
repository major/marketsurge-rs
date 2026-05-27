//! Browser cookie authentication for MarketSurge.

use crate::{Client, ClientError};

use super::error::render_client_error;

/// Build a MarketSurge client from browser cookies and a JWT exchange.
#[cfg(not(any(test, coverage)))]
pub async fn make_client() -> Result<Client, i32> {
    match Client::from_browser().await {
        Ok(client) => Ok(client),
        Err(err) => Err(render_client_error(&err)),
    }
}

/// Build a test client without reading browser cookies.
#[cfg(any(test, coverage))]
pub async fn make_client() -> Result<Client, i32> {
    Client::new(crate::ClientConfig::default()).map_err(handle_api_error)
}

/// Convert API errors into CLI exit codes and messages.
pub fn handle_api_error(err: ClientError) -> i32 {
    render_client_error(&err)
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
    fn test_handle_api_error_auth_returns_auth_error() {
        assert_eq!(handle_api_error(status_error(401)), 4);
    }

    #[test]
    fn test_handle_api_error_other_returns_api_error() {
        assert_eq!(handle_api_error(status_error(500)), 3);
    }
}
