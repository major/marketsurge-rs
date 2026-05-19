//! Shared command execution helpers.

use std::future::Future;

use marketsurge_client::Client;
use serde::Serialize;

use crate::common::auth::{handle_api_error, make_client};
use crate::output::{finish_output, print_json};

/// Runs a command through the standard client/output lifecycle.
///
/// Handles client creation and output formatting. The caller provides a
/// closure that receives the [`Client`] and returns serializable records.
///
/// Use [`run_command`] instead when the command also needs symbol references.
///
/// # Errors
///
/// Returns a non-zero exit code if client creation fails or the closure
/// returns an error code.
pub async fn run_client_command<T, F, Fut>(json_table: bool, execute: F) -> i32
where
    T: Serialize,
    F: FnOnce(Client) -> Fut,
    Fut: Future<Output = Result<Vec<T>, i32>>,
{
    let client = match make_client().await {
        Ok(c) => c,
        Err(code) => return code,
    };

    match execute(client).await {
        Ok(records) => finish_output(print_json(&records, json_table)),
        Err(code) => code,
    }
}

/// Runs a symbol-based command through the standard lifecycle.
///
/// Handles client creation, symbol reference conversion, and output
/// formatting. The caller provides a closure that performs API call(s) and
/// transforms the response into serializable records.
///
/// This is a convenience wrapper around [`run_client_command`] that
/// converts owned symbol strings into borrowed references.
///
/// # Errors
///
/// Returns a non-zero exit code if client creation fails or the closure
/// returns an error code.
pub async fn run_command<'a, T, F, Fut>(symbols: &'a [String], json_table: bool, execute: F) -> i32
where
    T: Serialize,
    F: FnOnce(Client, Vec<&'a str>) -> Fut,
    Fut: Future<Output = Result<Vec<T>, i32>>,
{
    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    run_client_command(json_table, |client| execute(client, symbol_refs)).await
}

/// Maps a client API future into the command error-code convention.
pub async fn api_call<T, Fut>(request: Fut) -> Result<T, i32>
where
    Fut: Future<Output = marketsurge_client::Result<T>>,
{
    request.await.map_err(handle_api_error)
}

/// Pairs symbols with response items by position.
///
/// Items beyond the symbol list length get `"???"` as a placeholder.
pub fn zip_symbols<'a, T>(
    symbols: &'a [&str],
    items: &'a [T],
) -> impl Iterator<Item = (&'a str, &'a T)> {
    items
        .iter()
        .enumerate()
        .map(move |(i, item)| (*symbols.get(i).unwrap_or(&"???"), item))
}

#[cfg(test)]
mod tests {
    use marketsurge_client::ClientError;

    use super::{api_call, zip_symbols};

    fn collect_pairs<'a, T>(symbols: &'a [&str], items: &'a [T]) -> Vec<(&'a str, &'a T)> {
        zip_symbols(symbols, items).collect()
    }

    #[test]
    fn test_zip_symbols_equal_length() {
        let symbols = ["AAPL", "MSFT"];
        let items = [1, 2];

        let zipped = collect_pairs(&symbols, &items);

        assert_eq!(zipped, vec![("AAPL", &1), ("MSFT", &2)]);
    }

    #[test]
    fn test_zip_symbols_more_items_than_symbols() {
        let symbols = ["AAPL"];
        let items = [1, 2, 3];

        let zipped = collect_pairs(&symbols, &items);

        assert_eq!(zipped, vec![("AAPL", &1), ("???", &2), ("???", &3)]);
    }

    #[test]
    fn test_zip_symbols_empty_symbols_non_empty_items() {
        let symbols: [&str; 0] = [];
        let items = [1, 2];

        let zipped = collect_pairs(&symbols, &items);

        assert_eq!(zipped, vec![("???", &1), ("???", &2)]);
    }

    #[test]
    fn test_zip_symbols_empty_items() {
        let symbols = ["AAPL"];
        let items: [i32; 0] = [];

        let zipped = collect_pairs(&symbols, &items);

        assert!(zipped.is_empty());
    }

    #[test]
    fn test_zip_symbols_both_empty() {
        let symbols: [&str; 0] = [];
        let items: [i32; 0] = [];

        let zipped = collect_pairs(&symbols, &items);

        assert_eq!(zipped, Vec::new());
    }

    #[tokio::test]
    async fn api_call_returns_success_value() {
        let result = api_call(async { Ok::<_, ClientError>(42) }).await;

        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn api_call_maps_client_error_to_exit_code() {
        let result = api_call(async {
            Err::<u32, _>(ClientError::Status {
                status: 500,
                body: "boom".to_string(),
            })
        })
        .await;

        assert_eq!(result, Err(1));
    }
}
