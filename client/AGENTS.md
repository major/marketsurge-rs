# AGENTS.md - marketsurge-client

Unofficial HTTP client library for the MarketSurge GraphQL API. Not affiliated with, endorsed by, or sponsored by IBD, MarketSurge, or Dow Jones. This is the core crate; `marketsurge-agent` depends on it.

## Module Layout

```text
client/src/
  lib.rs              public API surface: Client, ClientConfig, ClientError, Result, 15 endpoint modules
  client.rs           core Client struct, ClientConfig builder, GraphQL request/response execution
  error.rs            thiserror-based ClientError (Status, BodyLimit, GraphQL, Http, Json)
  auth.rs             JWT token exchange from session cookies
  browser_auth.rs     Firefox cookie extraction via rookie crate
  graphql.rs          GraphQL request/response envelope types
  graphql/            embedded GraphQL operation documents used by include_str!
  types.rs            shared domain types (date wrappers, numeric types)

  # Endpoint modules (one per MarketSurge API operation)
  adhoc_screen.rs     ad-hoc stock screening
  chart.rs            chart/price data
  coach.rs            pattern recognition coaching
  fundamentals.rs     fundamental stock data
  industry.rs         industry group rankings
  market_data.rs      broad market data
  nav.rs              navigation/menu data
  ownership.rs        institutional ownership
  ratings.rs          stock ratings
  screen.rs           saved stock screens
  watchlist.rs        watchlist management

  test_support.rs     test-only helpers (gated behind #[cfg(test)])
```

## Public API

The crate's public surface is defined in `lib.rs`:
- `Client` - main API client, constructed via `ClientConfig`
- `ClientConfig` - builder for client configuration (base URL, cookies, body limit)
- `ClientError` - error enum for all failure modes
- `Result<T>` - type alias for `std::result::Result<T, ClientError>`
- Each endpoint module re-exports its response/request types

## Key Patterns

### Client Construction

`ClientConfig` uses a builder pattern: `ClientConfig::new()` with `.base_url()`, `.cookies()`, `.body_limit()` setters, then `.build()` to produce a `Client`.

### GraphQL Execution

All API calls go through `Client::graphql_post()`, which sends a POST with a `GraphQLRequest` body and parses the `GraphQLResponse<T>` envelope. Endpoint modules keep typed methods thin by using the crate-private `Client::graphql_operation()` helper. GraphQL documents live under `client/src/graphql/` and are embedded with `include_str!`, so the crate has no runtime file dependency.

### Error Handling

`ClientError` uses `thiserror` derive:
- `Status` - non-2xx HTTP response
- `BodyLimit` - response body exceeds configured limit
- `GraphQL` - API returned errors in the GraphQL response
- `Http` - reqwest transport errors
- `Json` - serde deserialization failures

### Wire Contract Fidelity

Response structs match the MarketSurge API shape exactly. Do not rename fields or restructure responses for "cleaner" Rust types. Use `#[serde(rename_all = "camelCase")]` and `#[serde(alias = "...")]` as needed.

`#[allow(clippy::too_many_arguments)]` in `adhoc_screen.rs` and `chart.rs` preserves the API's parameter surface. Do not refactor these signatures.

## Dependencies

| Crate | Purpose |
|---|---|
| `reqwest` | HTTP client (cookies, json features) |
| `rookie` | Browser cookie extraction |
| `chrono` | Date/time types with serde support |
| `serde` + `serde_json` | JSON serialization/deserialization |
| `thiserror` | Error derive macros |
| `tokio` | async runtime (full features) |
| `tracing` | structured logging |
| `url` | URL parsing |

Dev dependencies: `mockito` (HTTP mocking), `tokio` (test-util)

## Testing

### Test Organization

All tests are colocated `#[cfg(test)] mod tests` at the bottom of each source file. No separate `tests/` directory.

### Test Helpers (`test_support.rs`)

Compiled only under `#[cfg(test)]`. Provides:
- `load_fixture(endpoint, file)` - reads JSON from `testdata/<endpoint>/<file>`
- `mock_graphql(server, response_body)` - sets up a mockito mock for the GraphQL endpoint
- `mock_graphql_response(server, status, body)` - sets up a raw GraphQL mock response for client error-path tests
- `mock_get_response(server, path, status, body)` - sets up a raw GET mock response for auth error-path tests
- `test_config(server)` - creates a `ClientConfig` pointed at the mock server
- `test_client(server)` - creates a `Client` pointed at the mock server
- `mock_test(operation)` - full mock setup for a single operation (loads fixture, creates mock, returns client)
- `mock_test_with_fixture(fixture_endpoint, operation)` - mock setup reusing another operation's fixture
- `live_client()` - creates a real client for live integration tests

### Fixture Convention

Fixtures live in `client/testdata/<Operation>/` with paired files:
- `request.json` - expected GraphQL request body
- `response.json` - mocked API response

Some tests reuse fixtures across operations (e.g., `ChartMarketDataWeekly` reuses `ChartMarketData`).

### Mocked Test Pattern

```rust
#[tokio::test]
async fn test_operation() {
    let (client, mock) = mock_test(Operation::SomeOp).await;
    let result = client.some_method().await.unwrap();
    assert!(!result.items.is_empty());
    mock.assert_async().await;
}
```

### Live Integration Tests

Live tests hit the real API and are excluded from normal test runs:
- Marked with `#[ignore]`
- Named `integration_*`
- Run via `make integration` (requires valid browser cookies)
- Use `live_client()` helper

## Security

- Never log or expose authentication cookies, tokens, or session data
- Cookie extraction (`browser_auth.rs`) reads from the local browser; never transmit cookies to third parties
- `auth.rs` handles JWT token exchange; tokens are ephemeral and not persisted
