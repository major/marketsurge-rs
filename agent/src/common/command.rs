//! Shared command execution helpers.

use std::future::Future;

use marketsurge_client::Client;
use serde::Serialize;

use crate::common::auth::make_client;
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
