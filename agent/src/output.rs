//! JSON output formatting and field selection.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::Value;

/// Writes `value` as compact JSON to stdout, newline-terminated.
///
/// When `json_table` is true, arrays of objects are converted to
/// array-of-arrays format with a header row before serialization.
pub fn print_json<T: Serialize>(value: &T, json_table: bool) -> io::Result<()> {
    if json_table {
        let v = serde_json::to_value(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        match &v {
            Value::Array(arr) if arr.first().is_some_and(Value::is_object) => {
                let table = values_to_table(arr);
                write_json(&mut io::stdout().lock(), &table)
            }
            _ => write_json(&mut io::stdout().lock(), &v),
        }
    } else {
        write_json(&mut io::stdout().lock(), value)
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

/// Converts an array of JSON objects into JSON Table format: an array whose
/// first element is the header row (field names) and remaining elements are
/// value rows. Headers are the union of all object keys, ordered by first
/// appearance across all rows. Missing keys in any row produce `null`.
fn values_to_table(records: &[Value]) -> Value {
    if !records.first().is_some_and(Value::is_object) {
        return Value::Array(records.to_vec());
    }

    let mut headers: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for record in records {
        if let Value::Object(obj) = record {
            for key in obj.keys() {
                if seen.insert(key.clone()) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let header_row = Value::Array(headers.iter().map(|h| Value::String(h.clone())).collect());

    let mut table = Vec::with_capacity(records.len() + 1);
    table.push(header_row);

    for record in records {
        if let Value::Object(obj) = record {
            let row: Vec<Value> = headers
                .iter()
                .map(|h| obj.get(h).cloned().unwrap_or(Value::Null))
                .collect();
            table.push(Value::Array(row));
        }
    }

    Value::Array(table)
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

    use super::{finish_output, values_to_table, write_json};

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
    fn values_to_table_converts_array_of_objects() {
        let records = sample_records();
        let values: Vec<serde_json::Value> = records
            .iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        let table = values_to_table(&values);
        let rows = table.as_array().unwrap();

        assert_eq!(rows.len(), 3, "header row + 2 data rows");

        let headers = rows[0].as_array().unwrap();
        assert!(headers.contains(&serde_json::Value::String("symbol".to_string())));
        assert!(headers.contains(&serde_json::Value::String("price".to_string())));

        let first_row = rows[1].as_array().unwrap();
        assert_eq!(first_row.len(), headers.len());
        assert!(first_row.contains(&serde_json::json!("AAPL")));
        assert!(first_row.contains(&serde_json::json!(150.5)));
    }

    #[test]
    fn values_to_table_returns_non_object_array_unchanged() {
        let values = vec![serde_json::json!(1), serde_json::json!(2)];
        let result = values_to_table(&values);
        assert_eq!(result, serde_json::json!([1, 2]));
    }

    #[test]
    fn values_to_table_handles_empty_array() {
        let result = values_to_table(&[]);
        assert_eq!(result, serde_json::json!([]));
    }
}
