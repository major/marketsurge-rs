# marketsurge-rs

[![CI](https://github.com/major/marketsurge-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/major/marketsurge-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/major/marketsurge-rs/graph/badge.svg)](https://codecov.io/gh/major/marketsurge-rs)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV: 1.95.0](https://img.shields.io/badge/MSRV-1.95.0-orange.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.95.0.html)

Unofficial Rust client library and CLI for querying market data from [MarketSurge](https://marketsurge.investors.com).

> **Disclaimer:** This project is not affiliated with, endorsed by, or sponsored by Investor's Business Daily (IBD), MarketSurge, or Dow Jones & Company. MarketSurge is a trademark of Dow Jones & Company. Use of this software is at your own risk.

## Workspace crates

| Crate | Description |
|---|---|
| [`marketsurge-client`](client/) | HTTP client library for the MarketSurge GraphQL API |
| [`marketsurge-agent`](agent/) | CLI binary for querying market data |

`marketsurge-agent` depends on `marketsurge-client`. The client crate has no dependency on the agent.

## Installation

### Pre-built binaries

Download a binary from the [latest release](https://github.com/major/marketsurge-rs/releases/latest). Builds are available for Linux, macOS, and Windows.

### cargo-binstall

```bash
cargo binstall marketsurge-agent
```

### Build from source

```bash
cargo install --path agent
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

# Generate shell completions
marketsurge-agent completions zsh > _marketsurge-agent
```

## Development

```bash
# Full check (fmt + clippy + test + docs)
make check

# Individual targets
make fmt        # cargo +nightly fmt
make clippy     # cargo clippy -- -D clippy::all
make test       # cargo test --workspace
make doc        # cargo doc with -D warnings

# Coverage (90% line minimum enforced)
make coverage

# Live integration tests (requires browser cookies)
make integration
```

## License

Apache-2.0
