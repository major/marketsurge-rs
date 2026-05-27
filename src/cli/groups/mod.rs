//! Command group modules that group related subcommands under a shared
//! top-level command and dispatch to the appropriate command handler.
//!
//! Each module's `Cmd` enum is `pub` because it is referenced in
//! `args.rs` (a sibling module) as `groups::<name>::Cmd`. These enums
//! become part of the public CLI surface through the top-level
//! `Commands` enum.

pub mod analysis;
pub mod auth;
pub mod industry;
pub mod market;
pub mod navigation;
pub mod ownership;
pub mod screen;
pub mod watchlist;
