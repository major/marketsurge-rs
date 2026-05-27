//! CLI schema introspection command.

use clap::{Command, CommandFactory, builder::StyledStr};
use serde::Serialize;

use crate::cli::Cli;
use crate::cli::common::error::{ERROR_SCHEMA, ErrorSchema};
use crate::cli::common::exit::{EXIT_CODES, ExitCodeMetadata};
use crate::cli::output::{finish_output, print_json};

/// Dumps the CLI command surface as compact JSON.
pub fn handle(fields: &[String]) -> i32 {
    finish_output(print_json(&schema_payload(), fields))
}

#[derive(Debug, Serialize)]
struct SchemaPayload {
    schema_version: u8,
    binary: &'static str,
    version: &'static str,
    exit_codes: &'static [ExitCodeMetadata],
    errors: ErrorSchema,
    commands: Vec<CommandSchema>,
}

#[derive(Debug, Serialize)]
struct CommandSchema {
    name: String,
    about: Option<String>,
    long_about: Option<String>,
    args: Vec<ArgSchema>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    subcommands: Vec<CommandSchema>,
}

#[derive(Debug, Serialize)]
struct ArgSchema {
    name: String,
    kind: &'static str,
    required: bool,
    default: Option<String>,
    help: Option<String>,
}

fn schema_payload() -> SchemaPayload {
    let cmd = Cli::command();
    let commands = cmd.get_subcommands().map(command_schema).collect();

    SchemaPayload {
        schema_version: 3,
        binary: "marketsurge-agent",
        version: env!("CARGO_PKG_VERSION"),
        exit_codes: EXIT_CODES,
        errors: ERROR_SCHEMA,
        commands,
    }
}

fn command_schema(command: &Command) -> CommandSchema {
    CommandSchema {
        name: command.get_name().to_string(),
        about: styled_str_to_string(command.get_about()),
        long_about: styled_str_to_string(command.get_long_about()),
        args: command
            .get_arguments()
            .filter(|arg| !arg.is_hide_set())
            .map(|arg| ArgSchema {
                name: arg.get_id().as_str().to_string(),
                kind: if arg.is_positional() {
                    "positional"
                } else {
                    "option"
                },
                required: arg.is_required_set(),
                default: arg
                    .get_default_values()
                    .first()
                    .map(|value| value.to_string_lossy().into_owned()),
                help: styled_str_to_string(arg.get_help()),
            })
            .collect(),
        subcommands: command.get_subcommands().map(command_schema).collect(),
    }
}

fn styled_str_to_string(value: Option<&StyledStr>) -> Option<String> {
    value.map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::schema_payload;

    #[test]
    fn payload_contains_top_level_metadata() {
        let payload = schema_payload();

        assert_eq!(payload.schema_version, 3);
        assert_eq!(payload.binary, "marketsurge-agent");
        assert_eq!(payload.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(payload.exit_codes.len(), 6);
        assert!(
            payload
                .commands
                .iter()
                .any(|command| command.name == "schema")
        );
    }

    #[test]
    fn payload_includes_exit_code_contract() {
        let payload = schema_payload();

        assert!(payload.exit_codes.iter().any(|entry| {
            entry.code == 2 && entry.name == "usage" && entry.description.contains("invalid")
        }));
        assert!(payload.exit_codes.iter().any(|entry| {
            entry.code == 4 && entry.name == "auth_error" && entry.description.contains("cookies")
        }));
    }

    #[test]
    fn payload_includes_structured_error_contract() {
        let payload = schema_payload();

        assert!(
            payload.errors.fields.iter().any(|field| {
                field.name == "kind" && field.required && field.r#type == "string"
            })
        );
        assert!(
            payload
                .errors
                .kinds
                .iter()
                .any(|kind| { kind.kind == "auth_error" && kind.exit_code == 4 })
        );
        assert!(
            payload
                .errors
                .kinds
                .iter()
                .any(|kind| { kind.kind == "rate_limit" && kind.exit_code == 3 })
        );
    }

    #[test]
    fn payload_includes_visible_command_arguments() {
        let payload = schema_payload();
        let analysis = payload
            .commands
            .iter()
            .find(|command| command.name == "analysis")
            .expect("analysis command should be present");
        let ratings = analysis
            .subcommands
            .iter()
            .find(|command| command.name == "ratings")
            .expect("ratings subcommand should be present");

        assert!(
            ratings
                .args
                .iter()
                .any(|arg| { arg.name == "symbols" && arg.kind == "positional" && arg.required }),
            "ratings should expose its required symbols positional arg"
        );
    }

    #[test]
    fn payload_includes_nested_visible_command_arguments() {
        let payload = schema_payload();
        let screen = payload
            .commands
            .iter()
            .find(|command| command.name == "screen")
            .expect("screen command should be present");
        let run = screen
            .subcommands
            .iter()
            .find(|command| command.name == "run")
            .expect("screen run command should be present");

        assert!(
            run.args
                .iter()
                .any(|arg| { arg.name == "screen_id" && arg.kind == "positional" && arg.required }),
            "screen run should expose its required screen_id positional arg"
        );
    }

    #[test]
    fn payload_documents_ownership_summary_float_pct_semantics() {
        let payload = schema_payload();
        let ownership = payload
            .commands
            .iter()
            .find(|command| command.name == "ownership")
            .expect("ownership command should be present");
        let summary = ownership
            .subcommands
            .iter()
            .find(|command| command.name == "summary")
            .expect("ownership summary command should be present");

        let long_about = summary
            .long_about
            .as_deref()
            .expect("ownership summary should have long help");
        assert!(long_about.contains("funds_float_pct_held"));
        assert!(long_about.contains("current percentage of float held by funds"));
        assert!(long_about.contains("does not provide this value per quarter"));
    }
}
