//! Structured CLI error rendering.

use std::sync::Mutex;

use clap::Error;
use serde::Serialize;

use crate::ClientError;

use super::exit::ExitCode;

static COMMAND_NAME: Mutex<Option<&'static str>> = Mutex::new(None);

/// Machine-readable metadata for a documented CLI error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ErrorKindMetadata {
    /// Stable symbolic error kind.
    pub kind: &'static str,
    /// Process exit code associated with this kind.
    pub exit_code: i32,
    /// Human-readable description of the condition.
    pub description: &'static str,
}

/// Machine-readable metadata for a structured error field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ErrorFieldMetadata {
    /// Field name in structured stderr JSON.
    pub name: &'static str,
    /// JSON type exposed by the field.
    pub r#type: &'static str,
    /// Whether every structured error includes this field.
    pub required: bool,
    /// Human-readable description of the field.
    pub description: &'static str,
}

/// Schema fragment describing structured CLI errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ErrorSchema {
    /// Fields that may appear in structured stderr JSON.
    pub fields: &'static [ErrorFieldMetadata],
    /// Stable error kinds emitted by the CLI.
    pub kinds: &'static [ErrorKindMetadata],
}

/// Structured stderr JSON shape emitted for CLI failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliError {
    /// Stable symbolic error kind.
    pub kind: &'static str,
    /// Human-readable error message.
    pub message: String,
    /// Process exit code returned by the command.
    pub exit_code: i32,
    /// HTTP status code when the error came from an HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Retry delay in seconds when the upstream service supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    /// Top-level command being executed when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<&'static str>,
    /// Actionable recovery hint when one is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Documented structured error fields.
pub const ERROR_FIELDS: &[ErrorFieldMetadata] = &[
    ErrorFieldMetadata {
        name: "kind",
        r#type: "string",
        required: true,
        description: "stable symbolic error kind",
    },
    ErrorFieldMetadata {
        name: "message",
        r#type: "string",
        required: true,
        description: "human-readable error message",
    },
    ErrorFieldMetadata {
        name: "exit_code",
        r#type: "integer",
        required: true,
        description: "process exit code returned by the command",
    },
    ErrorFieldMetadata {
        name: "status_code",
        r#type: "integer",
        required: false,
        description: "HTTP status code when available",
    },
    ErrorFieldMetadata {
        name: "retry_after",
        r#type: "integer",
        required: false,
        description: "retry delay in seconds when available",
    },
    ErrorFieldMetadata {
        name: "command",
        r#type: "string",
        required: false,
        description: "top-level command being executed when known",
    },
    ErrorFieldMetadata {
        name: "suggestion",
        r#type: "string",
        required: false,
        description: "actionable recovery hint when available",
    },
];

/// Documented structured error kinds.
pub const ERROR_KINDS: &[ErrorKindMetadata] = &[
    ErrorKindMetadata {
        kind: "warning",
        exit_code: 0,
        description: "non-fatal diagnostic emitted on stderr while command output still succeeds",
    },
    ErrorKindMetadata {
        kind: "usage",
        exit_code: 2,
        description: "invalid arguments or command usage",
    },
    ErrorKindMetadata {
        kind: "auth_error",
        exit_code: 4,
        description: "browser cookies are missing, expired, or rejected",
    },
    ErrorKindMetadata {
        kind: "api_error",
        exit_code: 3,
        description: "network failure or upstream MarketSurge API failure",
    },
    ErrorKindMetadata {
        kind: "rate_limit",
        exit_code: 3,
        description: "upstream MarketSurge API rate limit response",
    },
    ErrorKindMetadata {
        kind: "internal_error",
        exit_code: 1,
        description: "unexpected internal error, including local output failures",
    },
    ErrorKindMetadata {
        kind: "no_results",
        exit_code: 5,
        description: "command completed but produced no actionable result",
    },
];

/// Schema fragment included by `marketsurge-agent schema`.
pub const ERROR_SCHEMA: ErrorSchema = ErrorSchema {
    fields: ERROR_FIELDS,
    kinds: ERROR_KINDS,
};

/// Record the top-level command for optional structured error context.
pub fn set_command_name(command: Option<&'static str>) {
    let mut current = COMMAND_NAME
        .lock()
        .expect("command name lock should not be poisoned");
    *current = command;
}

/// Render a clap usage error and return its exit code.
pub fn render_usage_error(err: &Error) -> i32 {
    let exit_code = err.exit_code();
    render_cli_error(&CliError {
        kind: "usage",
        message: err.to_string().trim().to_string(),
        exit_code,
        status_code: None,
        retry_after: None,
        command: None,
        suggestion: Some("Run with --help to see valid commands and options.".to_string()),
    })
}

/// Render a command usage error and return its mapped exit code.
pub fn render_usage_message(message: String) -> i32 {
    render_usage_message_with_suggestion(message, None)
}

/// Render a command usage error with an actionable suggestion.
pub fn render_usage_message_with_suggestion(message: String, suggestion: Option<String>) -> i32 {
    render_cli_error(&CliError {
        kind: "usage",
        message,
        exit_code: ExitCode::Usage.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion,
    })
}

/// Render a non-fatal structured warning and return success.
pub fn render_warning_message(message: String) -> i32 {
    render_cli_error(&CliError {
        kind: "warning",
        message,
        exit_code: ExitCode::Success.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion: Some("Check the column names and spelling before retrying."),
    })
}

/// Render a client error and return its mapped exit code.
pub fn render_client_error(err: &ClientError) -> i32 {
    render_cli_error(&structured_client_error(err))
}

/// Render a non-client API error and return its mapped exit code.
pub fn render_api_error(message: String) -> i32 {
    render_cli_error(&CliError {
        kind: "api_error",
        message,
        exit_code: ExitCode::ApiError.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion: None,
    })
}

/// Render an internal CLI error and return its mapped exit code.
pub fn render_internal_error(message: String) -> i32 {
    render_cli_error(&CliError {
        kind: "internal_error",
        message,
        exit_code: ExitCode::InternalError.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion: None,
    })
}

/// Render a no-results CLI error and return its mapped exit code.
pub fn render_no_results_error(message: &'static str) -> i32 {
    render_no_results_message(message.to_string(), None)
}

/// Render a no-results CLI error with an optional actionable suggestion.
pub fn render_no_results_message(message: String, suggestion: Option<String>) -> i32 {
    render_cli_error(&CliError {
        kind: "no_results",
        message,
        exit_code: ExitCode::NoResults.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion,
    })
}

/// Convert a no-results condition into the documented structured error shape.
#[must_use]
pub fn structured_no_results_error(message: &'static str) -> CliError {
    CliError {
        kind: "no_results",
        message: message.to_string(),
        exit_code: ExitCode::NoResults.code(),
        status_code: None,
        retry_after: None,
        command: command_name(),
        suggestion: None,
    }
}

/// Convert a client error into the documented structured error shape.
#[must_use]
pub fn structured_client_error(err: &ClientError) -> CliError {
    let (kind, exit_code, suggestion) = if err.is_auth_error() {
        (
            "auth_error",
            ExitCode::AuthError.code(),
            Some("Log in to MarketSurge in Firefox, then retry the command.".to_string()),
        )
    } else if err.is_rate_limited() {
        (
            "rate_limit",
            ExitCode::ApiError.code(),
            Some("Wait before retrying the request.".to_string()),
        )
    } else {
        ("api_error", ExitCode::ApiError.code(), None)
    };

    CliError {
        kind,
        message: err.to_string(),
        exit_code,
        status_code: err.status_code(),
        retry_after: err.retry_after().map(|duration| duration.as_secs()),
        command: command_name(),
        suggestion,
    }
}

fn render_cli_error(error: &CliError) -> i32 {
    let mut stderr = std::io::stderr().lock();
    let _ = serde_json::to_writer(&mut stderr, error);
    use std::io::Write;
    let _ = stderr.write_all(b"\n");

    error.exit_code
}

fn command_name() -> Option<&'static str> {
    *COMMAND_NAME
        .lock()
        .expect("command name lock should not be poisoned")
}

#[cfg(test)]
mod tests {
    use clap::Command;

    use crate::ClientError;

    use super::{
        ERROR_SCHEMA, render_api_error, render_client_error, render_internal_error,
        render_no_results_error, render_usage_error, render_usage_message, render_warning_message,
        set_command_name, structured_client_error, structured_no_results_error,
    };

    fn status_error(status: u16) -> ClientError {
        ClientError::Status {
            status,
            body: "boom".to_string(),
        }
    }

    #[test]
    fn structured_client_error_maps_auth_error() {
        let error = structured_client_error(&status_error(401));

        assert_eq!(error.kind, "auth_error");
        assert_eq!(error.exit_code, 4);
        assert_eq!(error.status_code, Some(401));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn structured_client_error_maps_rate_limit_error() {
        let error = structured_client_error(&status_error(429));

        assert_eq!(error.kind, "rate_limit");
        assert_eq!(error.exit_code, 3);
        assert_eq!(error.status_code, Some(429));
    }

    #[test]
    fn structured_client_error_maps_generic_api_error() {
        let error = structured_client_error(&status_error(500));

        assert_eq!(error.kind, "api_error");
        assert_eq!(error.exit_code, 3);
        assert_eq!(error.status_code, Some(500));
        assert!(error.suggestion.is_none());
    }

    #[test]
    fn structured_no_results_error_uses_no_results_contract() {
        let error = structured_no_results_error("nothing matched");

        assert_eq!(error.kind, "no_results");
        assert_eq!(error.message, "nothing matched");
        assert_eq!(error.exit_code, 5);
        assert!(error.status_code.is_none());
    }

    #[test]
    fn structured_errors_include_current_command_name() {
        set_command_name(Some("screen"));

        let error = structured_no_results_error("nothing matched");

        assert_eq!(error.command, Some("screen"));

        set_command_name(None);
    }

    #[test]
    fn render_usage_error_returns_clap_exit_code() {
        let err = Command::new("marketsurge-agent")
            .try_get_matches_from(["marketsurge-agent", "--definitely-invalid"])
            .expect_err("invalid flag should fail parsing");

        assert_eq!(render_usage_error(&err), 2);
    }

    #[test]
    fn render_helpers_return_documented_exit_codes() {
        set_command_name(Some("market"));

        assert_eq!(render_usage_message("invalid input".to_string()), 2);
        assert_eq!(render_warning_message("invalid input".to_string()), 0);
        assert_eq!(render_api_error("upstream failed".to_string()), 3);
        assert_eq!(render_internal_error("broken pipe".to_string()), 1);
        assert_eq!(render_no_results_error("nothing matched"), 5);

        set_command_name(None);
    }

    #[test]
    fn render_client_error_returns_structured_client_exit_code() {
        assert_eq!(render_client_error(&status_error(401)), 4);
        assert_eq!(render_client_error(&status_error(500)), 3);
    }

    #[test]
    fn schema_documents_required_error_fields_and_kinds() {
        assert!(
            ERROR_SCHEMA.fields.iter().any(|field| {
                field.name == "kind" && field.required && field.r#type == "string"
            })
        );
        assert!(
            ERROR_SCHEMA
                .kinds
                .iter()
                .any(|kind| { kind.kind == "rate_limit" && kind.exit_code == 3 })
        );
        assert!(
            ERROR_SCHEMA
                .kinds
                .iter()
                .any(|kind| { kind.kind == "warning" && kind.exit_code == 0 })
        );
    }
}
