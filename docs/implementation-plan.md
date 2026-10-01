# Implementation Plan

## Repository inspection (2026-10-01)

| Area | Status |
|------|--------|
| `Cargo.toml` | Missing — greenfield |
| `src/` | Missing |
| `tests/` | Missing |
| `examples/` | Missing |
| `README.md` | Present — product direction and state model documented |
| Persistence | None |
| State enum | None in code |
| CI | None |
| `.gitignore` | Missing |

README describes planned layout (`ingestion/`, `execution/`, etc.). This implementation uses the layered layout from the task spec while preserving README lifecycle semantics.

## Phase 1 — Crate skeleton

- `Cargo.toml`: library + binary, deps (`rust_decimal`, `thiserror`, `tracing`, `serde`, `chrono`, `tokio`, `rusqlite`, `uuid`)
- Optional feature `polymarket-sdk` → `polymarket_client_sdk_v2` (read-oriented adapter, no order placement)
- `.gitignore`, GitHub Actions CI
- Module tree under `src/`

## Phase 2 — Domain

- `identifiers.rs`: `ExecutionId`, `OrderId`, `TradeId`, `EventId`, etc.
- `states.rs`: `ExecutionState`, `VerificationState`, `PositionCheckResult`, terminal-state helpers
- `execution.rs`: `ExecutionRecord` with quantities (`Decimal`), timestamps, idempotency keys
- `events.rs`: normalized observation events + telemetry event names
- `verification.rs`: `VerificationResult`, `PositionMismatch`

## Phase 3 — State machine

- `verifier/transitions.rs`: allowed transitions, illegal transition errors, no regression from terminal verified states
- `verifier/policies.rs`: when to set UNKNOWN / RETRYING / INCONSISTENT
- `verifier/engine.rs`: apply events, aggregate partial fills, idempotency via trade/event IDs

## Phase 4 — Gateway & reconciliation

- Traits: `ExecutionSource`, `TransactionSource`, `PositionSource` in `gateway/mod.rs`
- `gateway/mock.rs`: deterministic scenarios for tests and example
- `gateway/polymarket.rs`: feature-gated mapping from SDK types (document field mapping in module docs)
- `reconciliation/executor.rs`: recovery flow (UNKNOWN/RECONCILING → authoritative queries → recompute)
- `reconciliation/position.rs`: expected vs observed comparison

## Phase 5 — Persistence

- `repository/mod.rs` trait
- `repository/memory.rs`
- `repository/sqlite.rs` + migrations

## Phase 6 — Tests & demo

- Unit tests in modules + integration tests under `tests/`
- `examples/basic_verification.rs` (scenarios A/B/C, no credentials)
- Update README to match implemented code

## SDK assumptions (to validate at compile time)

- Crate: `polymarket_client_sdk_v2` with `data` feature for positions/trades
- Adapter is read-only; live credentials not required for default build/tests
- Exact response fields mapped in `gateway/polymarket.rs` module documentation

## Intentionally out of scope

- Live order submission
- Trading strategy logic
- HTTP API (no existing server structure)
- Postgres (trait left open via `repository`)
