//! JSON output formatting and field selection.

use std::collections::BTreeSet;
use std::io::{self, Write};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::cli::common::error::{
    render_internal_error, render_usage_message_with_suggestion, render_warning_message,
};

const INVALID_FIELDS_SUGGESTION: &str =
    "Run `marketsurge-agent schema` to discover valid top-level output fields.";

/// Output formatting failures mapped to structured CLI diagnostics.
#[derive(Debug)]
pub enum OutputError {
    /// Local JSON serialization or stdout write failure.
    Io(io::Error),
    /// The user requested fields that do not exist in the selected output.
    InvalidFields(Vec<String>),
}

/// Writes `value` as compact JSON to stdout, newline-terminated.
pub fn print_json<T: Serialize>(value: &T, fields: &[String]) -> Result<(), OutputError> {
    let selected_fields = selected_fields(fields);
    if selected_fields.is_empty() {
        write_json(&mut io::stdout().lock(), value).map_err(OutputError::Io)
    } else {
        let value = serde_json::to_value(value)
            .map_err(|e| OutputError::Io(io::Error::new(io::ErrorKind::InvalidData, e)))?;
        validate_selected_fields(&value, &selected_fields)?;
        let filtered = filter_value_fields(value, &selected_fields);
        write_json(&mut io::stdout().lock(), &filtered).map_err(OutputError::Io)
    }
}

/// Convert an output write result into the CLI exit code convention.
pub fn finish_output(result: Result<(), OutputError>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(OutputError::Io(err)) => render_internal_error(err.to_string()),
        Err(OutputError::InvalidFields(fields)) => render_usage_message_with_suggestion(
            invalid_fields_message(&fields),
            Some(INVALID_FIELDS_SUGGESTION.to_string()),
        ),
    }
}

fn selected_fields(fields: &[String]) -> Vec<&str> {
    fields
        .iter()
        .map(|field| field.trim())
        .filter(|field| !field.is_empty())
        .collect()
}

fn filter_value_fields(value: Value, fields: &[&str]) -> Value {
    match value {
        Value::Object(object) => Value::Object(filter_object_fields(object, fields)),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| match value {
                    Value::Object(object) => Value::Object(filter_object_fields(object, fields)),
                    other => other,
                })
                .collect(),
        ),
        other => other,
    }
}

fn validate_selected_fields(value: &Value, fields: &[&str]) -> Result<(), OutputError> {
    let available_fields = available_object_fields(value);
    if available_fields.is_empty() {
        return Ok(());
    }

    let missing = missing_fields(fields, &available_fields);
    if missing.is_empty() {
        Ok(())
    } else if missing.len() == fields.len() {
        Err(OutputError::InvalidFields(missing))
    } else {
        render_warning_message(
            partial_invalid_fields_message(&missing),
            Some(INVALID_FIELDS_SUGGESTION.to_string()),
        );
        Ok(())
    }
}

fn available_object_fields(value: &Value) -> BTreeSet<&str> {
    match value {
        Value::Object(object) => object.keys().map(String::as_str).collect(),
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|object| object.keys().map(String::as_str))
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn missing_fields(fields: &[&str], available_fields: &BTreeSet<&str>) -> Vec<String> {
    fields
        .iter()
        .filter(|field| !available_fields.contains(**field))
        .map(|field| (*field).to_string())
        .collect()
}

fn invalid_fields_message(fields: &[String]) -> String {
    format!("unrecognized --fields name(s): {}", fields.join(", "))
}

fn partial_invalid_fields_message(fields: &[String]) -> String {
    format!(
        "{}. Valid fields in the same request were still returned.",
        invalid_fields_message(fields)
    )
}

fn filter_object_fields(mut object: Map<String, Value>, fields: &[&str]) -> Map<String, Value> {
    fields
        .iter()
        .filter_map(|field| {
            object
                .remove(*field)
                .map(|value| ((*field).to_string(), value))
        })
        .collect()
}

/// Writes `value` as compact JSON to `writer`, newline-terminated.
fn write_json<W: Write, T: Serialize>(writer: &mut W, value: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde::Serialize;

    use crate::cli::common::exit::ExitCode;

    use super::{
        available_object_fields, filter_value_fields, finish_output, invalid_fields_message,
        missing_fields, partial_invalid_fields_message, print_json, selected_fields,
        validate_selected_fields, write_json,
    };

    #[derive(Debug, Serialize)]
    struct TestRecord {
        symbol: String,
        price: f64,
    }

    fn sample_records() -> Vec<TestRecord> {
        vec![
            TestRecord {
                symbol: "AAPL".to_string(),
                price: 150.5,
            },
            TestRecord {
                symbol: "MSFT".to_string(),
                price: 320.75,
            },
        ]
    }

    #[test]
    fn output_compact_json() {
        let record = &sample_records()[0];
        let mut buf = Vec::new();
        write_json(&mut buf, record).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1, "compact JSON should be a single line");
        assert!(output.ends_with('\n'));

        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(parsed["symbol"], "AAPL");
        assert_eq!(parsed["price"], 150.5);
    }

    #[test]
    fn finish_output_maps_result_to_exit_code() {
        assert_eq!(finish_output(Ok(())), 0);
        assert_eq!(
            finish_output(Err(super::OutputError::Io(std::io::Error::other(
                "broken pipe"
            )))),
            ExitCode::InternalError.code()
        );
        assert_eq!(
            finish_output(Err(super::OutputError::InvalidFields(vec![
                "missing".to_string()
            ]))),
            ExitCode::Usage.code()
        );
    }

    #[test]
    fn print_json_writes_all_fields_by_default() {
        let record = &sample_records()[0];

        print_json(record, &[]).unwrap();
    }

    #[test]
    fn print_json_applies_selected_fields() {
        let records = sample_records();
        let fields = vec!["symbol".to_string()];

        print_json(&records, &fields).unwrap();
    }

    #[test]
    fn selected_fields_omits_empty_names() {
        let fields = vec!["symbol".to_string(), String::new(), "price".to_string()];

        assert_eq!(selected_fields(&fields), vec!["symbol", "price"]);
    }

    #[test]
    fn selected_fields_trims_whitespace() {
        let fields = vec![
            " symbol".to_string(),
            "  ".to_string(),
            "price ".to_string(),
        ];

        assert_eq!(selected_fields(&fields), vec!["symbol", "price"]);
    }

    #[test]
    fn filter_value_fields_limits_array_of_objects() {
        let records = sample_records();
        let value = serde_json::to_value(records).unwrap();

        let filtered = filter_value_fields(value, &["symbol"]);

        assert_eq!(
            filtered,
            serde_json::json!([
                {"symbol": "AAPL"},
                {"symbol": "MSFT"}
            ])
        );
    }

    #[test]
    fn filter_value_fields_limits_single_object() {
        let value = serde_json::json!({"symbol": "AAPL", "price": 150.5});

        let filtered = filter_value_fields(value, &["price"]);

        assert_eq!(filtered, serde_json::json!({"price": 150.5}));
    }

    #[test]
    fn filter_value_fields_omits_missing_fields() {
        let value = serde_json::json!({"symbol": "AAPL"});

        let filtered = filter_value_fields(value, &["missing"]);

        assert_eq!(filtered, serde_json::json!({}));
    }

    #[test]
    fn available_object_fields_unions_array_object_keys() {
        let value = serde_json::json!([
            {"symbol": "AAPL", "price": 150.5},
            {"symbol": "MSFT", "volume": 1000}
        ]);

        let available = available_object_fields(&value);

        assert!(available.contains("symbol"));
        assert!(available.contains("price"));
        assert!(available.contains("volume"));
    }

    #[test]
    fn available_object_fields_is_empty_for_scalar_output() {
        let value = serde_json::json!(4);

        let available = available_object_fields(&value);

        assert!(available.is_empty());
    }

    #[test]
    fn validate_selected_fields_accepts_scalar_and_valid_object_fields() {
        let scalar = serde_json::json!(4);
        let object = serde_json::json!({"schema_version": 4});

        validate_selected_fields(&scalar, &["anything"]).unwrap();
        validate_selected_fields(&object, &["schema_version"]).unwrap();
    }

    #[test]
    fn validate_selected_fields_warns_for_partial_invalid_fields() {
        let object = serde_json::json!({"schema_version": 4});

        validate_selected_fields(&object, &["schema_version", "missing"]).unwrap();
    }

    #[test]
    fn validate_selected_fields_rejects_all_invalid_fields() {
        let object = serde_json::json!({"schema_version": 4});

        let err = validate_selected_fields(&object, &["missing", "other"]).unwrap_err();

        assert!(matches!(
            err,
            super::OutputError::InvalidFields(fields) if fields == vec!["missing", "other"]
        ));
    }

    #[test]
    fn missing_fields_reports_requested_names_absent_from_output() {
        let available = BTreeSet::from(["symbol", "price"]);

        let missing = missing_fields(&["symbol", "missing", "other"], &available);

        assert_eq!(missing, vec!["missing", "other"]);
    }

    #[test]
    fn invalid_fields_messages_match_structured_warning_style() {
        let fields = vec!["missing".to_string(), "other".to_string()];

        assert_eq!(
            invalid_fields_message(&fields),
            "unrecognized --fields name(s): missing, other"
        );
        assert_eq!(
            partial_invalid_fields_message(&fields),
            "unrecognized --fields name(s): missing, other. Valid fields in the same request were still returned."
        );
    }

    #[test]
    fn filter_value_fields_leaves_non_object_array_values_unchanged() {
        let value = serde_json::json!([1, {"symbol": "AAPL", "price": 150.5}]);

        let filtered = filter_value_fields(value, &["symbol"]);

        assert_eq!(filtered, serde_json::json!([1, {"symbol": "AAPL"}]));
    }

    #[test]
    fn filter_value_fields_leaves_scalar_values_unchanged() {
        let value = serde_json::json!("AAPL");

        let filtered = filter_value_fields(value, &["symbol"]);

        assert_eq!(filtered, serde_json::json!("AAPL"));
    }
}
