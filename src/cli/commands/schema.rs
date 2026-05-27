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
    output_fields: Vec<OutputFieldSchema>,
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

#[derive(Debug, Clone, Copy, Serialize)]
struct OutputFieldSchema {
    name: &'static str,
    r#type: &'static str,
    required: bool,
    description: &'static str,
}

const ANALYSIS_FUNDAMENTALS_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field("company_name", "string", false, "company name"),
    output_field(
        "metric",
        "string",
        true,
        "metric branch: reported_eps, reported_sales, eps_estimate, or sales_estimate",
    ),
    output_field(
        "period_offset",
        "string",
        false,
        "period offset, such as CURRENT or P1Q_AGO",
    ),
    output_field(
        "period",
        "string",
        false,
        "period end date or estimate period identifier",
    ),
    output_field("value", "string", false, "formatted metric value"),
    output_field(
        "pct_change_yoy",
        "string",
        false,
        "formatted year-over-year percent change",
    ),
    output_field(
        "revision_direction",
        "string",
        false,
        "estimate revision direction for EPS estimates",
    ),
];

const ANALYSIS_RATINGS_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field("period", "string", false, "rating period"),
    output_field("period_offset", "string", false, "rating period offset"),
    output_field("letter_value", "string", false, "letter rating value"),
    output_field("value", "integer", false, "numeric RS rating value"),
    output_field(
        "rs_line_new_high",
        "boolean",
        false,
        "whether the RS line is at a new high",
    ),
];

const MARKET_CHART_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field("period", "string", true, "time series period"),
    output_field("date", "string", true, "period start timestamp"),
    output_field("open", "number", false, "opening price"),
    output_field("high", "number", false, "period high price"),
    output_field("low", "number", false, "period low price"),
    output_field("close", "number", false, "closing price"),
    output_field("volume", "number", false, "trading volume"),
];

const MARKET_SNAPSHOT_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field("company_name", "string", false, "company name"),
    output_field("instrument_type", "string", false, "instrument subtype"),
    output_field("ipo_date", "string", false, "IPO date"),
    output_field("comp_rating", "integer", false, "Composite Rating"),
    output_field("rs_rating", "integer", false, "Relative Strength rating"),
    output_field("eps_rating", "integer", false, "EPS rating"),
    output_field("smr_rating", "string", false, "SMR rating letter"),
    output_field(
        "ad_rating",
        "string",
        false,
        "Accumulation/Distribution rating letter",
    ),
    output_field(
        "market_cap",
        "string",
        false,
        "formatted market capitalization",
    ),
    output_field(
        "avg_dollar_volume_50d",
        "string",
        false,
        "formatted 50-day average dollar volume",
    ),
    output_field(
        "up_down_volume_ratio",
        "string",
        false,
        "formatted up/down volume ratio",
    ),
    output_field(
        "short_interest_pct_float",
        "string",
        false,
        "formatted short interest as percent of float",
    ),
    output_field(
        "short_interest_days_to_cover",
        "string",
        false,
        "formatted short interest days to cover",
    ),
    output_field("industry_name", "string", false, "industry group name"),
    output_field("industry_sector", "string", false, "industry sector"),
    output_field(
        "industry_stocks_in_group",
        "integer",
        false,
        "number of stocks in the industry group",
    ),
    output_field(
        "funds_pct_float_held",
        "string",
        false,
        "formatted fund ownership percent of float",
    ),
    output_field("eps_due_date", "string", false, "next EPS due date"),
    output_field(
        "eps_due_date_status",
        "string",
        false,
        "EPS due date status",
    ),
    output_field("debt_pct", "string", false, "formatted debt percent"),
    output_field(
        "rd_pct_last_qtr",
        "string",
        false,
        "formatted R&D percent last quarter",
    ),
    output_field(
        "decode_error",
        "string",
        false,
        "per-symbol market data decode error",
    ),
];

const OWNERSHIP_SUMMARY_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field(
        "funds_float_pct_held",
        "string",
        false,
        "current percentage of float held by funds",
    ),
    output_field("date", "string", false, "quarter date"),
    output_field(
        "num_funds_held",
        "string",
        false,
        "number of funds holding the stock",
    ),
];

const OWNERSHIP_FUNDS_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field(
        "queried_symbol",
        "string",
        true,
        "stock ticker that was queried",
    ),
    output_field("fund_symbol", "string", false, "fund ticker symbol"),
    output_field("fund_name", "string", false, "fund name"),
    output_field(
        "holdings_pct",
        "string",
        false,
        "holdings as percent of fund assets held",
    ),
    output_field(
        "shares_held_1q_ago",
        "string",
        false,
        "shares held one quarter ago",
    ),
    output_field(
        "date_1q_ago",
        "string",
        false,
        "date for one quarter ago holdings",
    ),
    output_field(
        "shares_held_2q_ago",
        "string",
        false,
        "shares held two quarters ago",
    ),
    output_field(
        "date_2q_ago",
        "string",
        false,
        "date for two quarters ago holdings",
    ),
    output_field(
        "shares_held_3q_ago",
        "string",
        false,
        "shares held three quarters ago",
    ),
    output_field(
        "date_3q_ago",
        "string",
        false,
        "date for three quarters ago holdings",
    ),
    output_field(
        "shares_held_4q_ago",
        "string",
        false,
        "shares held four quarters ago",
    ),
    output_field(
        "date_4q_ago",
        "string",
        false,
        "date for four quarters ago holdings",
    ),
];

const INDUSTRY_RS_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("symbol", "string", true, "ticker symbol"),
    output_field("group_rs", "integer", false, "industry group RS value"),
];

const INDUSTRY_OVERVIEW_OUTPUT_FIELDS: &[OutputFieldSchema] = &[
    output_field("ticker", "string", true, "requested ticker symbol"),
    output_field(
        "industry_id",
        "string",
        true,
        "MarketSurge industry identifier",
    ),
    output_field("name", "string", false, "industry group name"),
    output_field("sector", "string", false, "sector name"),
    output_field("ind_code", "integer", false, "numeric industry code"),
    output_field(
        "group_market_value_billions",
        "string",
        false,
        "formatted group market value in billions",
    ),
    output_field(
        "num_new_highs",
        "integer",
        false,
        "number of stocks at new highs",
    ),
    output_field(
        "num_new_lows",
        "integer",
        false,
        "number of stocks at new lows",
    ),
    output_field(
        "num_stocks",
        "integer",
        false,
        "total number of stocks in the group",
    ),
    output_field("group_rank", "integer", false, "current group rank"),
    output_field(
        "pct_change_1d",
        "string",
        false,
        "formatted price percent change vs 1 day ago",
    ),
    output_field(
        "pct_change_ytd",
        "string",
        false,
        "formatted price percent change year to date",
    ),
    output_field(
        "eps_rank",
        "integer",
        false,
        "EPS rank within industry group",
    ),
    output_field("rs_rank", "integer", false, "RS rank within industry group"),
    output_field(
        "ad_rank",
        "integer",
        false,
        "Accumulation/Distribution rank within industry group",
    ),
    output_field(
        "smr_rank",
        "integer",
        false,
        "SMR rank within industry group",
    ),
    output_field(
        "comp_rank",
        "integer",
        false,
        "Composite rank within industry group",
    ),
];

const fn output_field(
    name: &'static str,
    r#type: &'static str,
    required: bool,
    description: &'static str,
) -> OutputFieldSchema {
    OutputFieldSchema {
        name,
        r#type,
        required,
        description,
    }
}

fn schema_payload() -> SchemaPayload {
    let cmd = Cli::command();
    let commands = cmd.get_subcommands().map(command_schema).collect();

    SchemaPayload {
        schema_version: 4,
        binary: "marketsurge-agent",
        version: env!("CARGO_PKG_VERSION"),
        exit_codes: EXIT_CODES,
        errors: ERROR_SCHEMA,
        commands,
    }
}

fn command_schema(command: &Command) -> CommandSchema {
    command_schema_with_path(command, &[])
}

fn command_schema_with_path(command: &Command, parent_path: &[&str]) -> CommandSchema {
    let name = command.get_name();
    let mut path = parent_path.to_vec();
    path.push(name);

    CommandSchema {
        name: name.to_string(),
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
        output_fields: output_fields_for_command(&path).to_vec(),
        subcommands: command
            .get_subcommands()
            .map(|subcommand| command_schema_with_path(subcommand, &path))
            .collect(),
    }
}

fn output_fields_for_command(path: &[&str]) -> &'static [OutputFieldSchema] {
    match path {
        ["analysis", "fundamentals"] => ANALYSIS_FUNDAMENTALS_OUTPUT_FIELDS,
        ["analysis", "ratings"] => ANALYSIS_RATINGS_OUTPUT_FIELDS,
        ["market", "chart"] => MARKET_CHART_OUTPUT_FIELDS,
        ["market", "snapshot"] => MARKET_SNAPSHOT_OUTPUT_FIELDS,
        ["ownership", "summary"] => OWNERSHIP_SUMMARY_OUTPUT_FIELDS,
        ["ownership", "funds"] => OWNERSHIP_FUNDS_OUTPUT_FIELDS,
        ["industry", "rs"] => INDUSTRY_RS_OUTPUT_FIELDS,
        ["industry", "overview"] => INDUSTRY_OVERVIEW_OUTPUT_FIELDS,
        _ => &[],
    }
}

fn styled_str_to_string(value: Option<&StyledStr>) -> Option<String> {
    value.map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::{output_field, output_fields_for_command, schema_payload};

    #[test]
    fn payload_contains_top_level_metadata() {
        let payload = schema_payload();

        assert_eq!(payload.schema_version, 4);
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

    #[test]
    fn payload_includes_screen_columns_command() {
        let payload = schema_payload();
        let screen = payload
            .commands
            .iter()
            .find(|command| command.name == "screen")
            .expect("screen command should be present");
        let columns = screen
            .subcommands
            .iter()
            .find(|command| command.name == "columns")
            .expect("screen columns command should be present");

        assert!(
            columns
                .about
                .as_deref()
                .is_some_and(|about| about.contains("discovered")),
            "screen columns should describe its discovery scope"
        );
    }

    #[test]
    fn payload_documents_output_fields_for_filterable_commands() {
        let payload = schema_payload();
        let analysis = payload
            .commands
            .iter()
            .find(|command| command.name == "analysis")
            .expect("analysis command should be present");
        let fundamentals = analysis
            .subcommands
            .iter()
            .find(|command| command.name == "fundamentals")
            .expect("analysis fundamentals command should be present");

        assert!(fundamentals.output_fields.iter().any(|field| {
            field.name == "pct_change_yoy" && field.r#type == "string" && !field.required
        }));

        let market = payload
            .commands
            .iter()
            .find(|command| command.name == "market")
            .expect("market command should be present");
        let chart = market
            .subcommands
            .iter()
            .find(|command| command.name == "chart")
            .expect("market chart command should be present");

        assert!(
            chart
                .output_fields
                .iter()
                .any(|field| field.name == "close" && field.r#type == "number")
        );
        assert!(
            chart
                .args
                .iter()
                .any(|arg| { arg.name == "days" && arg.kind == "option" && !arg.required })
        );
        assert!(
            chart
                .args
                .iter()
                .any(|arg| { arg.name == "start_date" && arg.kind == "option" && !arg.required })
        );
    }

    #[test]
    fn output_field_constructor_preserves_metadata() {
        let field = output_field("example", "string", true, "example description");

        assert_eq!(field.name, "example");
        assert_eq!(field.r#type, "string");
        assert!(field.required);
        assert_eq!(field.description, "example description");
    }

    #[test]
    fn output_fields_for_command_is_empty_for_non_filterable_command() {
        assert!(output_fields_for_command(&["schema"]).is_empty());
    }
}
