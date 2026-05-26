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

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn combined_output(output: &Output) -> String {
    format!("{}{}", stdout(output), stderr(output))
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
    assert_eq!(schema["schema_version"], 1);
    assert_eq!(schema["binary"], "marketsurge-agent");
    assert_eq!(schema["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        schema["commands"]
            .as_array()
            .is_some_and(|commands| { commands.iter().any(|command| command["name"] == "schema") })
    );

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
        serde_json::json!({"schema_version": 1, "binary": "marketsurge-agent"})
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
    assert!(
        stderr(&output).contains("unexpected argument '--definitely-invalid'"),
        "invalid flag should use clap's error path"
    );
}

#[test]
#[cfg_attr(coverage, ignore)]
fn missing_subcommand_returns_exit_code_2() {
    let output = output(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        combined_output(&output).contains("Usage: marketsurge-agent [OPTIONS] <COMMAND>"),
        "missing subcommand should print help"
    );
}
