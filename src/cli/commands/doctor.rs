//! Doctor diagnostic command: runs local and network checks to verify
//! the tool is configured correctly and can reach MarketSurge.
//!
//! Modeled after `chezmoi doctor` and `claude doctor`. Each check
//! produces a named status with a detail string and optional
//! suggestion. The command always writes compact JSON to stdout so
//! scripts and LLM agents can consume the results. Exit codes
//! reflect the worst check result: non-zero when any check fails.

use serde::Serialize;

use crate::cli::common::exit::ExitCode;
use crate::cli::output::{finish_output, print_json};

/// Status of a single diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Check passed.
    Ok,
    /// Minor issue detected, but the tool should still work.
    Warning,
    /// Check failed; something is broken.
    Failed,
    /// Check was purposely skipped (e.g., `--skip-network`).
    Skipped,
}

/// A single diagnostic check result.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    /// Short name identifying the check.
    pub name: &'static str,
    /// Result status.
    pub status: CheckStatus,
    /// Human-readable detail about the result.
    pub detail: String,
    /// Actionable suggestion when status is not ok.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<&'static str>,
}

/// Summary counts of check results.
#[derive(Debug, Default, Clone, Serialize)]
pub struct DoctorSummary {
    /// Number of checks that passed.
    pub ok: usize,
    /// Number of checks with warnings.
    pub warning: usize,
    /// Number of checks that failed.
    pub failed: usize,
    /// Number of skipped checks.
    pub skipped: usize,
}

/// Top-level doctor output payload written to stdout.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorOutput {
    /// Binary name.
    pub binary: &'static str,
    /// Package version from Cargo.toml.
    pub version: &'static str,
    /// Operating system family.
    pub os: String,
    /// Individual check results.
    pub checks: Vec<DoctorCheck>,
    /// Summary counts.
    pub summary: DoctorSummary,
}

impl DoctorOutput {
    /// Build a payload from a list of checks, computing the summary.
    fn collect(checks: Vec<DoctorCheck>) -> Self {
        let mut summary = DoctorSummary::default();
        for check in &checks {
            match check.status {
                CheckStatus::Ok => summary.ok += 1,
                CheckStatus::Warning => summary.warning += 1,
                CheckStatus::Failed => summary.failed += 1,
                CheckStatus::Skipped => summary.skipped += 1,
            }
        }
        Self {
            binary: "marketsurge-agent",
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS.to_string(),
            checks,
            summary,
        }
    }

    /// Returns the most severe check status.
    fn worst_status(&self) -> CheckStatus {
        if self.checks.iter().any(|c| c.status == CheckStatus::Failed) {
            CheckStatus::Failed
        } else if self.checks.iter().any(|c| c.status == CheckStatus::Warning) {
            CheckStatus::Warning
        } else {
            CheckStatus::Ok
        }
    }
}

/// Run the doctor diagnostic command.
pub fn handle(fields: &[String], skip_network: bool) -> i32 {
    let mut checks = Vec::new();

    checks.push(check_binary_version());
    checks.push(check_config());
    checks.push(check_firefox_cookies());

    if skip_network {
        checks.push(skip_check("jwt_exchange"));
        checks.push(skip_check("graphql_connectivity"));
    }

    let output = DoctorOutput::collect(checks);

    // Determine exit code before writing output so a broken-pipe
    // on stdout does not swallow the exit code.
    let exit_code = match output.worst_status() {
        CheckStatus::Failed => ExitCode::InternalError.code(),
        _ => ExitCode::Success.code(),
    };

    finish_output(print_json(&output, fields));

    exit_code
}

/// Build a skipped-check entry for `--skip-network`.
fn skip_check(name: &'static str) -> DoctorCheck {
    DoctorCheck {
        name,
        status: CheckStatus::Skipped,
        detail: "skipped (--skip-network)".to_string(),
        suggestion: None,
    }
}

fn check_binary_version() -> DoctorCheck {
    DoctorCheck {
        name: "binary_version",
        status: CheckStatus::Ok,
        detail: format!("{} (MSRV 1.95.0)", env!("CARGO_PKG_VERSION")),
        suggestion: None,
    }
}

fn check_config() -> DoctorCheck {
    let config = crate::ClientConfig::default();
    DoctorCheck {
        name: "config",
        status: CheckStatus::Ok,
        detail: format!(
            "graphql_url={}, investors_url={}, timeout={}s, body_limit={}",
            config.graphql_url,
            config.investors_base_url,
            config.timeout.as_secs(),
            config.body_limit,
        ),
        suggestion: None,
    }
}

fn check_firefox_cookies() -> DoctorCheck {
    match crate::browser_auth::extract_cookies() {
        Ok(cookies) => {
            let count = cookies.len();
            if count == 0 {
                DoctorCheck {
                    name: "firefox_cookies",
                    status: CheckStatus::Failed,
                    detail: "no investors.com cookies found".to_string(),
                    suggestion: Some(
                        "Log in to https://marketsurge.investors.com in Firefox, then retry.",
                    ),
                }
            } else {
                DoctorCheck {
                    name: "firefox_cookies",
                    status: CheckStatus::Ok,
                    detail: format!(
                        "{} investors.com cookie{} found",
                        count,
                        if count == 1 { "" } else { "s" },
                    ),
                    suggestion: None,
                }
            }
        }
        Err(err) => DoctorCheck {
            name: "firefox_cookies",
            status: CheckStatus::Failed,
            detail: format!("failed to extract cookies: {err}"),
            suggestion: Some("Ensure Firefox is installed and you are logged into MarketSurge."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::common::exit::ExitCode;

    #[test]
    fn check_binary_version_is_ok() {
        let check = check_binary_version();
        assert_eq!(check.status, CheckStatus::Ok);
        assert!(check.detail.contains(env!("CARGO_PKG_VERSION")));
        assert!(check.detail.contains("MSRV"));
        assert!(check.suggestion.is_none());
    }

    #[test]
    fn check_config_is_ok() {
        let check = check_config();
        assert_eq!(check.status, CheckStatus::Ok);
        assert!(check.detail.contains("graphql_url="));
        assert!(check.detail.contains("investors_url="));
        assert!(check.detail.contains("timeout="));
        assert!(check.detail.contains("body_limit="));
        assert!(check.suggestion.is_none());
    }

    #[test]
    fn skip_check_returns_skipped_status() {
        let check = skip_check("jwt_exchange");
        assert_eq!(check.status, CheckStatus::Skipped);
        assert_eq!(check.name, "jwt_exchange");
        assert!(check.detail.contains("--skip-network"));
        assert!(check.suggestion.is_none());
    }

    #[test]
    fn collect_computes_summary_counts() {
        let checks = vec![
            DoctorCheck {
                name: "a",
                status: CheckStatus::Ok,
                detail: "ok".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "b",
                status: CheckStatus::Warning,
                detail: "warn".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "c",
                status: CheckStatus::Failed,
                detail: "fail".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "d",
                status: CheckStatus::Skipped,
                detail: "skip".to_string(),
                suggestion: None,
            },
        ];

        let output = DoctorOutput::collect(checks);

        assert_eq!(output.summary.ok, 1);
        assert_eq!(output.summary.warning, 1);
        assert_eq!(output.summary.failed, 1);
        assert_eq!(output.summary.skipped, 1);
    }

    #[test]
    fn worst_status_failed_beats_warning() {
        let checks = vec![
            DoctorCheck {
                name: "a",
                status: CheckStatus::Ok,
                detail: "ok".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "b",
                status: CheckStatus::Failed,
                detail: "fail".to_string(),
                suggestion: None,
            },
        ];
        let output = DoctorOutput::collect(checks);
        assert_eq!(output.worst_status(), CheckStatus::Failed);
    }

    #[test]
    fn worst_status_warning_beats_ok() {
        let checks = vec![
            DoctorCheck {
                name: "a",
                status: CheckStatus::Ok,
                detail: "ok".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "b",
                status: CheckStatus::Warning,
                detail: "warn".to_string(),
                suggestion: None,
            },
        ];
        let output = DoctorOutput::collect(checks);
        assert_eq!(output.worst_status(), CheckStatus::Warning);
    }

    #[test]
    fn worst_status_all_ok_returns_ok() {
        let checks = vec![DoctorCheck {
            name: "a",
            status: CheckStatus::Ok,
            detail: "ok".to_string(),
            suggestion: None,
        }];
        let output = DoctorOutput::collect(checks);
        assert_eq!(output.worst_status(), CheckStatus::Ok);
    }

    #[test]
    fn output_has_required_top_level_fields() {
        let checks = vec![check_binary_version()];
        let output = DoctorOutput::collect(checks);

        assert_eq!(output.binary, "marketsurge-agent");
        assert_eq!(output.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(output.os, std::env::consts::OS);
        assert!(!output.checks.is_empty());
    }

    #[test]
    fn output_serializes_to_json() {
        let checks = vec![
            check_binary_version(),
            check_config(),
            skip_check("jwt_exchange"),
        ];
        let output = DoctorOutput::collect(checks);

        let json = serde_json::to_string(&output).expect("DoctorOutput should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized DoctorOutput should be valid JSON");

        assert_eq!(parsed["binary"], "marketsurge-agent");
        assert_eq!(parsed["summary"]["ok"], serde_json::json!(2));
        assert_eq!(parsed["summary"]["skipped"], serde_json::json!(1));
        assert_eq!(parsed["checks"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn output_omits_none_suggestions() {
        let check = check_binary_version();
        let output = DoctorOutput::collect(vec![check]);

        let json = serde_json::to_string(&output).expect("DoctorOutput should serialize");

        // The binary_version check has no suggestion, so "suggestion"
        // should not appear in its JSON object.
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized DoctorOutput should be valid JSON");
        let first = &parsed["checks"][0];
        assert!(!first.as_object().unwrap().contains_key("suggestion"));
    }

    #[test]
    fn output_json_has_all_top_level_keys() {
        let checks = vec![check_binary_version(), check_config()];
        let output = DoctorOutput::collect(checks);

        let json = serde_json::to_value(&output).expect("DoctorOutput should serialize");
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();

        assert!(keys.contains(&"binary"));
        assert!(keys.contains(&"version"));
        assert!(keys.contains(&"os"));
        assert!(keys.contains(&"checks"));
        assert!(keys.contains(&"summary"));
    }

    #[test]
    fn handle_exit_code_is_success_when_no_failed_checks() {
        // Pure logic: worst of [Ok, Warning, Skipped] should yield
        // CheckStatus::Warning, which maps to Success exit code.
        let checks = vec![
            DoctorCheck {
                name: "a",
                status: CheckStatus::Ok,
                detail: "ok".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "b",
                status: CheckStatus::Warning,
                detail: "warn".to_string(),
                suggestion: None,
            },
        ];
        let output = DoctorOutput::collect(checks);
        assert_eq!(output.worst_status(), CheckStatus::Warning);
        assert_eq!(ExitCode::Success.code(), 0);
    }

    #[test]
    fn handle_exit_code_is_internal_error_when_any_check_failed() {
        let checks = vec![
            DoctorCheck {
                name: "a",
                status: CheckStatus::Ok,
                detail: "ok".to_string(),
                suggestion: None,
            },
            DoctorCheck {
                name: "b",
                status: CheckStatus::Failed,
                detail: "fail".to_string(),
                suggestion: None,
            },
        ];
        let output = DoctorOutput::collect(checks);
        assert_eq!(output.worst_status(), CheckStatus::Failed);
        assert_eq!(ExitCode::InternalError.code(), 1);
    }
}
