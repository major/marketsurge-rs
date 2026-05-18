//! Async Rust client for the MarketSurge GraphQL API.
//!
//! This project is unofficial and is not affiliated with, endorsed by, or
//! sponsored by [MarketSurge](https://marketsurge.investors.com).
//!
//! # Quick start
//!
//! ```no_run
//! use marketsurge_client::{Client, ClientConfig};
//!
//! # fn main() -> marketsurge_client::Result<()> {
//! let config = ClientConfig::default();
//! let client = Client::new(config)?;
//! // Use any endpoint method, e.g. client.chart_market_data(...).await
//! # Ok(())
//! # }
//! ```
//!
//! An agent crate providing higher-level workflows is planned separately.

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
pub mod markups;
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
