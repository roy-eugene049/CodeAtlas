# CodeAtlas Engineering Rules

## Architecture

CodeAtlas is a Rust-first application.

Domain logic must not depend on HTTP, database,
LLM providers, or frontend concerns.

## Rust

Prefer:
- Result<T, E>
- thiserror for library/domain errors
- anyhow only at application boundaries
- traits for replaceable infrastructure
- immutable data where practical
- explicit ownership

Avoid:
- unwrap()
- expect()
- excessive cloning
- global mutable state
- giant structs
- giant functions

## Async

Tokio is the async runtime.

Never perform blocking filesystem,
Git, or CPU-heavy operations directly
inside async request handlers.

CPU-heavy work must use appropriate
blocking/thread-pool mechanisms.

Concurrency must be bounded.

## API

Axum handlers should remain thin.

Handler:
request
→ validation
→ service
→ response

Business logic belongs in services.

## Testing

Every core subsystem requires tests.

Parser changes require parser fixtures.

Graph changes require graph tests.

Indexer changes require integration tests.

## AI

LLMs never receive arbitrary entire repositories.

Context must be retrieved and ranked.

AI responses must cite source locations.

Never send secrets or .env contents to an LLM.

## Performance

Every significant performance claim must
have a benchmark.

Never claim an optimization without measurement.