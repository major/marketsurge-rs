//! Stable CLI exit-code contract.

use serde::Serialize;

/// Machine-readable metadata for a documented CLI exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExitCodeMetadata {
    /// Numeric process exit code.
    pub code: i32,
    /// Stable symbolic name for scripts and schema consumers.
    pub name: &'static str,
    /// Human-readable description of the condition.
    pub description: &'static str,
}

/// Stable process exit codes for `marketsurge-agent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Command completed successfully.
    Success,
    /// Unexpected internal error, including local output failures.
    InternalError,
    /// Invalid arguments or command usage.
    Usage,
    /// Network failure, rate limit, or upstream API failure.
    ApiError,
    /// Browser cookies are missing, expired, or rejected.
    AuthError,
    /// Command completed but produced no actionable result.
    NoResults,
}

impl ExitCode {
    /// Return the numeric process code.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::InternalError => 1,
            Self::Usage => 2,
            Self::ApiError => 3,
            Self::AuthError => 4,
            Self::NoResults => 5,
        }
    }
}

/// Exit-code table shown in help, README, man pages, and schema output.
pub const EXIT_CODES: &[ExitCodeMetadata] = &[
    ExitCodeMetadata {
        code: 0,
        name: "success",
        description: "command completed successfully",
    },
    ExitCodeMetadata {
        code: 1,
        name: "internal_error",
        description: "unexpected internal error, including local output failures",
    },
    ExitCodeMetadata {
        code: 2,
        name: "usage",
        description: "invalid arguments or command usage",
    },
    ExitCodeMetadata {
        code: 3,
        name: "api_error",
        description: "network failure, rate limit, or upstream MarketSurge API failure",
    },
    ExitCodeMetadata {
        code: 4,
        name: "auth_error",
        description: "browser cookies are missing, expired, or rejected",
    },
    ExitCodeMetadata {
        code: 5,
        name: "no_results",
        description: "command completed but produced no actionable result",
    },
];

/// Help text appended to the top-level CLI help and generated man page.
pub const EXIT_CODE_HELP: &str = "Exit codes:\n  0  success - command completed successfully\n  1  internal_error - unexpected internal error, including local output failures\n  2  usage - invalid arguments or command usage\n  3  api_error - network failure, rate limit, or upstream MarketSurge API failure\n  4  auth_error - browser cookies are missing, expired, or rejected\n  5  no_results - command completed but produced no actionable result";

#[cfg(test)]
mod tests {
    use super::{EXIT_CODES, ExitCode};

    #[test]
    fn exit_code_values_are_stable() {
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::InternalError.code(), 1);
        assert_eq!(ExitCode::Usage.code(), 2);
        assert_eq!(ExitCode::ApiError.code(), 3);
        assert_eq!(ExitCode::AuthError.code(), 4);
        assert_eq!(ExitCode::NoResults.code(), 5);
    }

    #[test]
    fn metadata_matches_enum_values() {
        let codes: Vec<i32> = EXIT_CODES.iter().map(|entry| entry.code).collect();

        assert_eq!(codes, vec![0, 1, 2, 3, 4, 5]);
    }
}
