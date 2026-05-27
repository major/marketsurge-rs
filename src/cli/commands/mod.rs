//! Command handlers for each CLI subcommand group.

/// Ad-hoc stock screening command.
pub mod adhoc_screen;
/// Auth: verify browser cookie and JWT readiness.
pub mod auth;
/// Chart OHLCV data command.
pub mod chart;
/// Shell completion generation.
pub mod completions;
/// Doctor: diagnostic checks for troubleshooting.
pub mod doctor;
/// Fundamental financial data command.
pub mod fundamentals;
/// Industry group data commands.
pub mod industry;
/// Broad market data command.
pub mod market_data;
/// Fund ownership data commands.
pub mod ownership;
/// RS rating data command.
pub mod ratings;
/// CLI schema introspection command.
pub mod schema;
/// Stock screen commands.
pub mod screen;
/// Navigation and coaching tree commands.
pub mod tree;
/// Watchlist data commands.
pub mod watchlist;
