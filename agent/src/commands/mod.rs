//! Command handlers for each CLI subcommand group.

/// Ad-hoc stock screening command.
pub mod adhoc_screen;
/// Chart OHLCV data command.
pub mod chart;
/// Shell completion generation.
pub mod completions;
/// Fundamental financial data command.
pub mod fundamentals;
/// Industry group data commands.
pub mod industry;
/// Broad market data command.
pub mod market_data;
/// Chart markup retrieval command.
pub mod markups;
/// Fund ownership data commands.
pub mod ownership;
/// RS rating data command.
pub mod ratings;
/// Stock screen commands.
pub mod screen;
/// Navigation and coaching tree commands.
pub mod tree;
/// Watchlist data commands.
pub mod watchlist;
