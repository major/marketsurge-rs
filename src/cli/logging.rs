//! Tracing subscriber initialization for CLI diagnostics.
//!
//! Wire `--verbose`, `--debug`, and `RUST_LOG` into a stderr
//! subscriber so agents can diagnose failures without reading the
//! source.  Cookie values, auth tokens, and full sensitive headers
//! are never logged.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Log level derived from CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Warnings and errors (default when no flag is set).
    Warn,
    /// Informational messages plus warnings and errors (`--verbose`).
    Info,
    /// Debug-level diagnostics (`--debug` or `-vv`).
    Debug,
}

/// Initialize the tracing subscriber for CLI diagnostics.
///
/// `verbose` is the `--verbose` repeat count (0 = silent, 1 = info,
/// 2+ = debug).  `debug` is the `--debug` boolean flag.  When
/// `RUST_LOG` is set in the environment it takes precedence over all
/// flag-derived levels.
///
/// All diagnostic output is written to stderr so it never contaminates
/// stdout JSON.
pub fn init(verbose: u8, debug: bool) {
    let filter = if let Ok(env) = std::env::var("RUST_LOG") {
        // RUST_LOG takes precedence over all CLI flags.
        tracing_subscriber::EnvFilter::try_new(env)
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
    } else {
        default_filter(level_from_flags(verbose, debug))
    };

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();
}

fn level_from_flags(verbose: u8, debug: bool) -> Level {
    if debug {
        Level::Debug
    } else {
        match verbose {
            0 => Level::Warn,
            1 => Level::Info,
            _ => Level::Debug,
        }
    }
}

fn default_filter(level: Level) -> tracing_subscriber::EnvFilter {
    let directive = match level {
        Level::Warn => "rusty_marketsurge=warn",
        Level::Info => "rusty_marketsurge=info",
        Level::Debug => "rusty_marketsurge=debug",
    };
    tracing_subscriber::EnvFilter::try_new(directive)
        .expect("default filter directive should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ord_is_correct() {
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
    }

    #[test]
    fn level_from_flags_no_args_is_warn() {
        assert_eq!(level_from_flags(0, false), Level::Warn);
    }

    #[test]
    fn level_from_flags_verbose_once_is_info() {
        assert_eq!(level_from_flags(1, false), Level::Info);
    }

    #[test]
    fn level_from_flags_verbose_twice_is_debug() {
        assert_eq!(level_from_flags(2, false), Level::Debug);
    }

    #[test]
    fn level_from_flags_debug_overrides_verbose() {
        assert_eq!(level_from_flags(0, true), Level::Debug);
    }

    #[test]
    fn level_from_flags_verbose_three_is_debug() {
        assert_eq!(level_from_flags(3, false), Level::Debug);
    }

    #[test]
    fn default_filter_directives_are_parseable() {
        let _ = default_filter(Level::Warn);
        let _ = default_filter(Level::Info);
        let _ = default_filter(Level::Debug);
    }
}
