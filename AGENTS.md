# AGENTS.md - marketsurge-rs

> **Keep AGENTS.md and README.md accurate.** When code changes affect structure, public API, conventions, or CI, update the relevant AGENTS.md (and README.md) in the same PR. Fix any discrepancies you find at the earliest opportunity.

## Overview

Unofficial Rust client library and CLI for the MarketSurge platform. Not affiliated with, endorsed by, or sponsored by IBD, MarketSurge, or Dow Jones. Single package `rusty-marketsurge` with a `cli` feature (enabled by default) that builds the `marketsurge-agent` binary.

The `marketsurge-agent schema` command dumps the CLI surface as experimental compact JSON for scripts and agent tooling. It uses Clap introspection, does not read browser cookies, and does not make network requests. `schema_version: 3` documents exit codes, structured stderr error fields, and error kinds.

## Package Layout

```text
marketsurge-rs/
  Cargo.toml          single package (rusty-marketsurge)
  Makefile            build automation
  .coderabbit.yaml    code review config
  build.rs            man page generation (cli feature only)
  dist-workspace.toml cargo-dist config
  rust-toolchain.toml pins Rust 1.95 for local builds
  src/                merged client + CLI source
  src/cli/            CLI modules (behind cli feature)
  testdata/           fixture files for mocked tests
  tests/              CLI smoke tests
  .github/workflows/  CI/CD pipelines
```

## Rust Conventions

- **Edition**: 2024
- **MSRV**: 1.95.0 (enforced in CI)
- **Formatting**: `cargo fmt` with nightly rustfmt in CI, no `.rustfmt.toml`
- **Clippy**: `-D clippy::all` baseline; CI adds `-A clippy::needless_borrow -A clippy::large_enum_variant`
- **Docs**: `RUSTDOCFLAGS="-D warnings"` (doc warnings are errors)
- **No config files**: no `.rustfmt.toml`, `clippy.toml`, `.editorconfig`, `deny.toml`, `.cargo/config.toml`, or `justfile`

## Makefile Targets

| Target | Purpose |
|---|---|
| `check` | fmt + clippy + test + doc (full local validation) |
| `fmt` | `cargo +nightly fmt` |
| `clippy` | `cargo clippy -- -D clippy::all` |
| `test` | `cargo test --workspace` |
| `integration` | `cargo test --workspace -- --ignored` (live API tests) |
| `doc` | `cargo doc` with `-D warnings` |
| `coverage` | `cargo llvm-cov --workspace --fail-under-lines 90` |
| `audit` | `cargo audit` |
| `clean` | `cargo clean` |

Run `make check` before opening PRs.

## CI/CD

- **ci.yml**: fmt (nightly), clippy (stable), test, MSRV check (1.95.0), coverage (90% line minimum), docs
- **cd.yml**: release-plz on main push and workflow_dispatch
- **audit.yml**: RustSec advisory checks on manifest changes + weekly schedule
- **release.yml**: cargo-dist tag-based releases (`v*.*.*` tags), creates GitHub releases

All workflow actions must use pinned versions. Workflows require minimum permissions.

## Coverage

90% line coverage enforced via `cargo llvm-cov --fail-under-lines 90`. Do not lower this threshold without justification.

## Testing

Unit and mocked tests are colocated `#[cfg(test)] mod tests` inside source files. CLI smoke tests live in `tests/`.

- **Unit tests**: `#[test]` for sync, `#[tokio::test]` for async
- **Mocked API tests**: use `mockito` with helpers from `src/test_support.rs`
- **Live tests**: `#[ignore]` + `integration_*` naming, run via `make integration`
- **Fixtures**: `testdata/<Operation>/` with `request.json` and `response.json` pairs

## Lint Suppressions

Existing suppressions (do not add new ones without justification):
- `#[allow(dead_code)]` in `src/test_support.rs` (test-only utilities)
- `#[allow(clippy::too_many_arguments)]` in `adhoc_screen.rs`, `chart.rs` (wire contract fidelity)
- `#[allow(missing_docs)]` in `market_data.rs` (should be fixed)

## Release Process

1. Merge to `main` triggers release-plz (cd.yml) for version bumps and changelog
2. Version tags (`v*.*.*`) trigger cargo-dist builds (release.yml)
3. cargo-dist produces platform binaries and creates GitHub releases
4. `Cargo.toml` has `[profile.dist]` with `lto = "thin"` for release builds
