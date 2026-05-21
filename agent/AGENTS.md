# AGENTS.md - marketsurge-agent

Unofficial CLI binary and library crate for the MarketSurge platform. Not affiliated with, endorsed by, or sponsored by IBD, MarketSurge, or Dow Jones. Depends on `marketsurge-client` for all API access.

## Module Layout

```text
agent/src/
  main.rs           binary entrypoint, calls marketsurge_agent::run()
  lib.rs            crate root, #![deny(missing_docs)], exports modules, run() dispatches commands
  cli.rs            Clap derive-based argument model (Cli struct, Commands enum, SymbolsArgs)
  output.rs         Compact JSON output formatting and top-level field projection
  commands/
    mod.rs          command module declarations
    chart.rs        OHLCV chart data (daily/weekly)
    completions.rs  shell completion generation
    fundamentals.rs fundamental financial data (EPS, sales, estimates)
    industry.rs     industry group RS + overview subcommands
    market_data.rs  broad market data snapshot
    ownership.rs    fund ownership summary + individual fund holdings
    ratings.rs      RS rating and relative strength data
    screen.rs       saved screen list + run
    watchlist.rs    watchlist list, symbols, screen subcommands
  common/
    mod.rs          shared utilities (auth, command helpers)
    auth.rs         browser cookie auth, error-to-exit-code mapping
    command.rs      run_command() harness, zip_symbols() helper
    rows.rs         row flattening and response column helpers
    test_support.rs shared command test constructors (cfg(test) only)
```

## Conventions

- **Doc enforcement**: `#![deny(missing_docs)]` in `lib.rs`. All public items must have doc comments.
- **Output contract**: stdout is compact machine-readable JSON with all fields by default. The global `--fields` option projects top-level JSON object fields after command data is built. Diagnostics and errors go to stderr via `tracing`.
- **Logging**: `tracing` + `tracing-subscriber` with env filter. CLI sets verbosity via `-v` flags.
- **Async runtime**: tokio with `macros`, `rt`, `rt-multi-thread` features.
- **Auth flow**: `common::auth::make_client()` handles cookie-based auth before any API call.
- **Command harness**: `common::command::run_command()` handles the client-creation/symbol-ref/output lifecycle and applies output field projection. New symbol-based commands provide a closure with the API call and transform logic.
- **Shared args**: `cli::SymbolsArgs` is the standard arg struct for commands that take ticker symbols. Embed via `#[command(flatten)]` if extra flags are needed.

## Dependencies

| Crate | Purpose |
|---|---|
| `marketsurge-client` | API client (path dependency) |
| `clap` + `clap_complete` | CLI parsing and shell completion generation |
| `serde` | JSON serialization for output |
| `tokio` | async runtime |
| `tracing` + `tracing-subscriber` | structured logging |

## Adding Commands

1. Add variant to `Commands` enum in `cli.rs` (use `SymbolsArgs` for symbol-based commands)
2. Add handler in `commands/` using `run_command()` from `common::command`
3. Wire dispatch in `lib.rs` `run()` match
4. Add doc comment to satisfy `#![deny(missing_docs)]`
