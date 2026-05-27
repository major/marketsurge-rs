//! Shared utilities used across command handlers.

/// Browser auth session bootstrapping.
pub mod auth;
/// Shared command execution helpers.
pub mod command;
/// Structured CLI error rendering.
pub mod error;
/// Stable process exit-code contract.
pub mod exit;
/// Shared tabular response helpers.
pub(crate) mod rows;
/// Test helpers shared across command modules.
#[cfg(test)]
pub(crate) mod test_support;
