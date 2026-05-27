use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn output_without_browser_cookies(args: &[&str]) -> Output {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("marketsurge-agent-empty-home-{unique}"));
    std::fs::create_dir_all(&home).expect("empty home directory should be created");

    let output = marketsurge_agent()
        .args(args)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("APPDATA", home.join("AppData"))
        .env("LOCALAPPDATA", home.join("AppData").join("Local"))
        .output()
        .expect("marketsurge-agent should run");

    std::fs::remove_dir_all(&home).expect("empty home directory should be removed");
    output
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn stderr_json(output: &Output) -> serde_json::Value {
    serde_json::from_str(&stderr(output)).expect("stderr should be valid JSON")
}

#[test]
fn help_returns_exit_code_0() {
    let output = output(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn completions_returns_exit_code_0() {
    let output = output(&["completions", "bash"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(&output).is_empty(),
        "completions should not write stderr"
    );
    assert!(stdout(&output).contains("marketsurge-agent"));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn schema_returns_exit_code_0_and_valid_json() {
    let output = output(&["schema"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).is_empty(), "schema should not write stderr");

    let schema: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(schema["schema_version"], 3);
    assert_eq!(schema["binary"], "marketsurge-agent");
    assert_eq!(schema["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        schema["exit_codes"],
        serde_json::json!([
            {
                "code": 0,
                "name": "success",
                "description": "command completed successfully"
            },
            {
                "code": 1,
                "name": "internal_error",
                "description": "unexpected internal error, including local output failures"
            },
            {
                "code": 2,
                "name": "usage",
                "description": "invalid arguments or command usage"
            },
            {
                "code": 3,
                "name": "api_error",
                "description": "network failure, rate limit, or upstream MarketSurge API failure"
            },
            {
                "code": 4,
                "name": "auth_error",
                "description": "browser cookies are missing, expired, or rejected"
            },
            {
                "code": 5,
                "name": "no_results",
                "description": "command completed but produced no actionable result"
            }
        ]),
        "schema should expose the full ordered exit-code contract"
    );
    assert!(
        schema["commands"]
            .as_array()
            .is_some_and(|commands| { commands.iter().any(|command| command["name"] == "schema") })
    );
    assert_eq!(schema["errors"]["fields"][0]["name"], "kind");
    assert!(schema["errors"]["kinds"].as_array().is_some_and(|kinds| {
        kinds
            .iter()
            .any(|kind| kind["kind"] == "rate_limit" && kind["exit_code"] == 3)
    }));
    assert!(schema["errors"]["kinds"].as_array().is_some_and(|kinds| {
        kinds
            .iter()
            .any(|kind| kind["kind"] == "warning" && kind["exit_code"] == 0)
    }));

    let line_count = stdout(&output).lines().count();
    assert_eq!(line_count, 1, "schema should be compact single-line JSON");
}

#[test]
#[cfg_attr(coverage, ignore)]
fn schema_honors_global_field_selection() {
    let output = output(&["--fields", "schema_version,binary", "schema"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).is_empty(), "schema should not write stderr");

    let schema: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(
        schema,
        serde_json::json!({"schema_version": 3, "binary": "marketsurge-agent"})
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn help_documents_exit_codes() {
    let output = output(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).contains("Exit codes:"),
        "help should include the exit-code contract"
    );
    assert!(
        stdout(&output).contains("4  auth_error"),
        "help should document auth failures separately from clap usage errors"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn invalid_flag_returns_exit_code_2() {
    let output = output(&["--definitely-invalid"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).is_empty(),
        "invalid flags should not write stdout"
    );
    let error = stderr_json(&output);
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["exit_code"], 2);
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("unexpected argument '--definitely-invalid'")),
        "usage error should preserve clap's message"
    );
    assert!(error["suggestion"].is_string());
}

#[test]
#[cfg_attr(coverage, ignore)]
fn invalid_query_writes_structured_stderr() {
    let output = output(&["screen", "adhoc", "--query", "{"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).is_empty(),
        "structured command usage errors should not write stdout"
    );

    let error = stderr_json(&output);
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["exit_code"], 2);
    assert_eq!(error["command"], "screen");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("invalid --query JSON")),
        "usage error should include the invalid query context"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn missing_subcommand_returns_exit_code_2() {
    let output = output(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).is_empty(),
        "usage errors should not write stdout"
    );

    let error = stderr_json(&output);
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["exit_code"], 2);
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("Usage: marketsurge-agent [OPTIONS] <COMMAND>")),
        "missing subcommand should include usage"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn missing_nested_subcommand_returns_exit_code_2() {
    let output = output(&["analysis"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).is_empty(),
        "usage errors should not write stdout"
    );

    let error = stderr_json(&output);
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["exit_code"], 2);
    assert!(
        error["message"].as_str().is_some_and(
            |message| message.contains("Usage: marketsurge-agent analysis [OPTIONS] <COMMAND>")
        ),
        "missing nested subcommands should include group usage"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn unknown_command_returns_exit_code_2() {
    let output = output(&["not-a-command"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout(&output).is_empty(),
        "unknown commands should not write stdout"
    );
    let error = stderr_json(&output);
    assert_eq!(error["kind"], "usage");
    assert_eq!(error["exit_code"], 2);
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("unrecognized subcommand 'not-a-command'")),
        "unknown commands should preserve clap's message"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn missing_browser_cookies_write_structured_stderr() {
    let output = output_without_browser_cookies(&["analysis", "ratings", "AAPL"]);

    assert_eq!(output.status.code(), Some(4));
    assert!(
        stdout(&output).is_empty(),
        "auth failures should not write stdout"
    );
    let error = stderr_json(&output);
    assert_eq!(error["kind"], "auth_error");
    assert_eq!(error["exit_code"], 4);
    assert_eq!(error["status_code"], 401);
    assert_eq!(error["command"], "analysis");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("no cookies found")),
        "auth error should preserve the underlying client message"
    );
    assert!(error["suggestion"].is_string());
}
