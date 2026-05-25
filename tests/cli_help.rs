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

#[test]
#[cfg_attr(coverage, ignore)]
fn top_level_help_exits_successfully() {
    let output = output(&["--help"]);

    assert!(output.status.success(), "help should exit 0");
    assert!(stderr(&output).is_empty(), "help should not write stderr");

    let stdout = stdout(&output);
    assert!(stdout.contains("Usage: marketsurge-agent [OPTIONS] <COMMAND>"));
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("ratings"));
    assert!(stdout.contains("watchlist"));
    assert!(stdout.contains("completions"));
    assert!(!stdout.contains("requires a subcommand"));
}

#[test]
#[cfg_attr(coverage, ignore)]
fn command_specific_help_exits_successfully() {
    for command in ["ratings", "chart", "market-data"] {
        let output = output(&[command, "--help"]);

        assert!(output.status.success(), "{command} --help should exit 0");
        assert!(
            stderr(&output).is_empty(),
            "{command} --help should not write stderr"
        );
        assert!(
            stdout(&output).contains(&format!("Usage: marketsurge-agent {command}")),
            "{command} --help should include command usage"
        );
    }
}

#[test]
#[cfg_attr(coverage, ignore)]
fn nested_command_help_exits_successfully() {
    let output = output(&["ownership", "summary", "--help"]);

    assert!(output.status.success(), "nested help should exit 0");
    assert!(
        stderr(&output).is_empty(),
        "nested help should not write stderr"
    );
    assert!(stdout(&output).contains("Usage: marketsurge-agent ownership summary"));
}
