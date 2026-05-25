# Releasing

This document describes the release workflow for `rusty-marketsurge`.

## Prerequisites

- `cargo-edit` installed: `cargo install cargo-edit --locked` (provides `cargo set-version`)
- `git-cliff` installed: `cargo install git-cliff --locked`
- GPG signing key configured in git (`git config user.signingkey`)
- crates.io Trusted Publisher configured (one-time setup, see below)

## Release Steps

### 1. Bump the version

```bash
cargo set-version <new-version>
cargo check
```

Use semver: patch for bug fixes, minor for new features, minor for breaking changes while still `0.x`.

### 2. Regenerate the changelog

```bash
git cliff --tag <new-version> -o CHANGELOG.md
```

Review `CHANGELOG.md` before committing. The `cliff.toml` filters conventional commits and skips release-preparation commits automatically.

### 3. Commit the release files

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -s -S -m "chore(release): prepare <new-version>"
```

Omit `Cargo.lock` from the staged files if it did not change.

### 4. Tag and push

```bash
git tag -s -a <new-version> -m "Release <new-version>"
git push
git push origin <new-version>
```

The tag push triggers `release.yml`, which builds multi-platform binaries via cargo-dist and publishes to crates.io via OIDC trusted publishing. No API token is stored anywhere.

## Semver policy

This project follows semver with a `0.x` convention: breaking changes bump the minor version (`0.3.0` to `0.4.0`), not the major. Only bump to `1.0.0` manually when the public API is stable.

## crates.io Trusted Publisher setup

Configure Trusted Publishing once in the crate settings on crates.io. No stored API token is needed after this.

| Field | Value |
|---|---|
| Owner | `major` |
| Repository | `marketsurge-rs` |
| Workflow file | `release.yml` |
| Package | `rusty-marketsurge` |

The `publish` job in `release.yml` uses `rust-lang/crates-io-auth-action` to exchange a GitHub OIDC token for a short-lived crates.io token. It requires `id-token: write` and `contents: read` permissions.

## What happens after the tag push

1. `release.yml` triggers on the version tag (`**[0-9]+.[0-9]+.[0-9]+*`)
2. cargo-dist runs `plan`, `build-local-artifacts`, `build-global-artifacts`, `host`, and `announce` jobs
3. The `publish` job authenticates via OIDC and runs `cargo publish`
4. A GitHub release is created with platform binaries attached

## Commit message convention

- Release commit: `chore(release): prepare <version>`
- Tag message: `Release <version>`
- The `cliff.toml` skip pattern (`^chore\(release\): prepare for`) excludes release commits from the changelog automatically
