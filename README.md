# marketsurge-rs

[![CI](https://github.com/major/marketsurge-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/major/marketsurge-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/major/marketsurge-rs/graph/badge.svg)](https://codecov.io/gh/major/marketsurge-rs)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV: 1.95.0](https://img.shields.io/badge/MSRV-1.95.0-orange.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.95.0.html)

Unofficial Rust client library and CLI for querying market data from [MarketSurge](https://marketsurge.investors.com).

> **Disclaimer:** This project is not affiliated with, endorsed by, or sponsored by Investor's Business Daily (IBD), MarketSurge, or Dow Jones & Company. MarketSurge is a trademark of Dow Jones & Company. Use of this software is at your own risk.

## Installation

### Pre-built binaries

Download a binary from the [latest release](https://github.com/major/marketsurge-rs/releases/latest). Builds are available for Linux, macOS, and Windows.

### cargo-binstall

```bash
cargo binstall rusty-marketsurge
```

### Build from source

```bash
cargo install rusty-marketsurge --locked
```

Requires Rust 1.95.0 or later.

## Usage

The CLI reads browser cookies from Firefox automatically for authentication. Log in to [MarketSurge](https://marketsurge.investors.com) in your browser first, then run commands.

Output goes to stdout as compact JSON with all fields included by default. Use `--fields` with a comma-delimited list to keep only selected top-level JSON fields. Pipe through `jq` for pretty-printing. Logs and errors go to stderr.

```bash
# Fund ownership summary for a stock
marketsurge-agent ownership summary AAPL

# Limit output to selected top-level fields
marketsurge-agent --fields symbol,num_funds_held ownership summary AAPL

# Find saved watchlists or screens by ID or name, including punctuation-insensitive matches like IBD 50
marketsurge-agent watchlist list --query ibd
marketsurge-agent screen list --query ibd

# Generate shell completions
marketsurge-agent completions zsh > _marketsurge-agent

# Check whether browser cookies and JWT are ready before running queries
marketsurge-agent auth status

# Dump the experimental CLI schema without network access
marketsurge-agent schema | jq '.commands | length'
```

### Schema introspection

`marketsurge-agent schema` dumps the CLI surface as compact JSON for scripts and agent tooling. It does not read browser cookies or make network requests. The schema shape is experimental; `schema_version: 2` includes the binary name, package version, exit-code metadata, command metadata, and visible command arguments.

### Exit codes

`marketsurge-agent` uses stable exit codes so scripts can distinguish usage, authentication, and upstream failures.

| Code | Name | Meaning |
|---:|---|---|
| 0 | `success` | Command completed successfully. |
| 1 | `internal_error` | Unexpected internal error, including local output failures. |
| 2 | `usage` | Invalid arguments or command usage. |
| 3 | `api_error` | Network failure, rate limit, or upstream MarketSurge API failure. |
| 4 | `auth_error` | Browser cookies are missing, expired, or rejected. |
| 5 | `no_results` | Command completed but produced no actionable result. |

The `schema` command includes the same table in its `exit_codes` field.

## Using as a library

Other Rust projects can depend on `rusty-marketsurge` as an API client without pulling in the CLI by disabling default features:

```toml
[dependencies]
rusty-marketsurge = { version = "0.4.0", default-features = false }
```

This excludes `clap` and `clap_complete` and exposes `Client`, `ClientConfig`, `ClientError`, and `Result`. The `cli` feature (enabled by default) adds the CLI parser and the `run` entry point used by the `marketsurge-agent` binary.

## Development

```bash
# Full check (fmt + clippy + test + docs)
make check

# Individual targets
make fmt        # cargo fmt --check
make fmt-fix    # cargo +nightly fmt --all
make clippy     # cargo clippy -- -D clippy::all
make test       # cargo test
make doc        # cargo doc with -D warnings

# Coverage (90% line minimum enforced)
make coverage

# Live integration tests (requires browser cookies)
make integration
```

## License

Apache-2.0
