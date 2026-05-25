use std::time::Duration;

/// Errors returned by the marketsurge client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP error {status}: {body}")]
    Status { status: u16, body: String },

    #[error("response body exceeded {limit} bytes (actual: {actual})")]
    BodyLimit { limit: usize, actual: usize },

    #[error("GraphQL errors: {errors:?}")]
    GraphQL {
        errors: Vec<crate::graphql::GraphQLFieldError>,
    },

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ClientError {
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn is_auth_error(&self) -> bool {
        matches!(self.status_code(), Some(401 | 403))
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self.status_code(), Some(429))
    }

    pub fn is_body_limit(&self) -> bool {
        matches!(self, Self::BodyLimit { .. })
    }

    pub fn retry_after(&self) -> Option<Duration> {
        None
    }
}

/// Convenience alias for results with [`ClientError`].
pub type Result<T> = std::result::Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_detects_401_and_403() {
        assert!(
            ClientError::Status {
                status: 401,
                body: String::new()
            }
            .is_auth_error()
        );
        assert!(
            ClientError::Status {
                status: 403,
                body: String::new()
            }
            .is_auth_error()
        );
    }

    #[test]
    fn auth_error_rejects_non_auth_status() {
        assert!(
            !ClientError::Status {
                status: 200,
                body: String::new()
            }
            .is_auth_error()
        );
    }

    #[test]
    fn rate_limit_detects_429() {
        assert!(
            ClientError::Status {
                status: 429,
                body: String::new()
            }
            .is_rate_limited()
        );
    }

    #[test]
    fn body_limit_detects_variant() {
        assert!(
            ClientError::BodyLimit {
                limit: 1024,
                actual: 0
            }
            .is_body_limit()
        );
    }

    #[test]
    fn status_code_returns_none_for_non_status_variants() {
        assert_eq!(
            ClientError::BodyLimit {
                limit: 1024,
                actual: 0
            }
            .status_code(),
            None
        );
        assert_eq!(
            ClientError::GraphQL { errors: Vec::new() }.status_code(),
            None
        );
    }

    #[test]
    fn retry_after_is_currently_unset() {
        assert_eq!(
            ClientError::Status {
                status: 429,
                body: String::new()
            }
            .retry_after(),
            None
        );
    }
}
