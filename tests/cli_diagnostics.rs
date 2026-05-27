use std::process::{Command, Output};

fn marketsurge_agent() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_marketsurge-agent"));
    command.env("CLAP_COLOR", "never").env("NO_COLOR", "1");
    command
}

fn output(args: &[&str]) -> Output {
    marketsurge_agent()
        .args(args)
        .output()
        .expect("marketsurge-agent should run")
}

fn output_with_env(args: &[&str], env_var: &str, env_val: &str) -> Output {
    marketsurge_agent()
        .args(args)
        .env(env_var, env_val)
        .output()
        .expect("marketsurge-agent should run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

// ── Flag parsing ──────────────────────────────────────────────────

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_flag_is_accepted() {
    let output = output(&["--verbose", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn debug_flag_is_accepted() {
    let output = output(&["--debug", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_short_form_is_accepted() {
    let output = output(&["-v", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_double_short_form_is_accepted() {
    let output = output(&["-vv", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_repeated_short_is_accepted() {
    let output = output(&["-v", "-v", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_can_precede_subcommands() {
    let output = output(&["--verbose", "--debug", "schema"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn rust_log_env_var_is_accepted() {
    let output = output_with_env(&["schema"], "RUST_LOG", "rusty_marketsurge=debug");

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn rust_log_invalid_filter_is_accepted() {
    let output = output_with_env(&["schema"], "RUST_LOG", "not a valid filter!!!!");

    assert_eq!(output.status.code(), Some(0));
}

// ── stdout integrity ──────────────────────────────────────────────

#[test]
#[cfg_attr(coverage, ignore)]
fn verbose_does_not_contaminate_stdout_json() {
    let output = output(&["--verbose", "schema"]);

    assert_eq!(output.status.code(), Some(0));

    let schema: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout should be valid JSON");
    assert_eq!(schema["binary"], "marketsurge-agent");

    let line_count = stdout(&output).lines().count();
    assert_eq!(line_count, 1, "stdout should be single-line compact JSON");
}

#[test]
#[cfg_attr(coverage, ignore)]
fn debug_does_not_contaminate_stdout_json() {
    let output = output(&["--debug", "schema"]);

    assert_eq!(output.status.code(), Some(0));

    let schema: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout should be valid JSON");
    assert_eq!(schema["binary"], "marketsurge-agent");

    let line_count = stdout(&output).lines().count();
    assert_eq!(line_count, 1, "stdout should be single-line compact JSON");
}

#[test]
#[cfg_attr(coverage, ignore)]
fn rust_log_does_not_contaminate_stdout_json() {
    let output = output_with_env(&["schema"], "RUST_LOG", "rusty_marketsurge=debug");

    assert_eq!(output.status.code(), Some(0));

    let schema: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout should be valid JSON");
    assert_eq!(schema["binary"], "marketsurge-agent");

    let line_count = stdout(&output).lines().count();
    assert_eq!(line_count, 1, "stdout should be single-line compact JSON");
}

// ── Help output ───────────────────────────────────────────────────

#[test]
#[cfg_attr(coverage, ignore)]
fn help_documents_verbose_and_debug_flags() {
    let output = output(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("--verbose"),
        "help should document --verbose flag"
    );
    assert!(
        stdout.contains("--debug"),
        "help should document --debug flag"
    );
    assert!(
        stdout.contains("RUST_LOG"),
        "help should document RUST_LOG env var"
    );
}
