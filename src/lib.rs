//! Async Rust client library and CLI for the MarketSurge GraphQL API.
//!
//! This project is unofficial and is not affiliated with, endorsed by, or
//! sponsored by [MarketSurge](https://marketsurge.investors.com).
//!
//! The HTTP client, endpoint modules, and error types are always available so other Rust
//! projects can consume the API without pulling in the CLI. The `cli` module and its
//! dependencies are gated behind the default `cli` feature.
//!
//! Library-only consumers should disable default features:
//!
//! ```toml
//! rusty-marketsurge = { version = "0.3.0", default-features = false }
//! ```
//!
//! # Quick start
//!
//! ```no_run
//! use rusty_marketsurge::{Client, ClientConfig};
//!
//! # fn main() -> rusty_marketsurge::Result<()> {
//! let config = ClientConfig::default();
//! let client = Client::new(config)?;
//! // Use any endpoint method, e.g. client.chart_market_data(...).await
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "cli")]
pub mod cli;

pub mod adhoc_screen;
pub mod auth;
pub mod browser_auth;
pub mod chart;
pub mod client;
pub mod coach;
pub mod error;
pub mod fundamentals;
pub mod graphql;
pub mod industry;
pub mod market_data;
pub mod nav;
pub mod ownership;
pub mod ratings;
pub mod screen;
pub mod types;
pub mod watchlist;

#[cfg(test)]
mod test_support;

pub use client::{Client, ClientConfig};
pub use error::{ClientError, Result};
