//! Command group modules that group related subcommands under a shared
//! top-level command and dispatch to the appropriate command handler.

// clippy really dislikes `pub(crate)` in favor of `pub`.
// Since these enums are used in `args.rs` (a sibling module), we
// re-export them via `pub(crate)` on each module to keep the public
// surface minimal.

pub mod analysis;
pub mod industry;
pub mod market;
pub mod navigation;
pub mod ownership;
pub mod screen;
pub mod watchlist;
