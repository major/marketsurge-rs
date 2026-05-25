//! JSON output formatting and field selection.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::{Map, Value};

/// Writes `value` as compact JSON to stdout, newline-terminated.
pub fn print_json<T: Serialize>(value: &T, fields: &[String]) -> io::Result<()> {
    let selected_fields = selected_fields(fields);
    if selected_fields.is_empty() {
        write_json(&mut io::stdout().lock(), value)
    } else {
        let value = serde_json::to_value(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let filtered = filter_value_fields(value, &selected_fields);
        write_json(&mut io::stdout().lock(), &filtered)
    }
}

/// Convert an output write result into the CLI exit code convention.
pub fn finish_output(result: io::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("output error: {err}");
            1
        }
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
    use serde::Serialize;

    use super::{filter_value_fields, finish_output, print_json, selected_fields, write_json};

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
        assert_eq!(finish_output(Err(std::io::Error::other("broken pipe"))), 1);
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
