# Polymarket Execution Verifier

> Verify Polymarket trade state across order, fill, transaction, confirmation, settlement, and position.

Substack: [A Polymarket Fill Isn’t the End: Verifying Execution and Settlement](https://casatrick.substack.com/p/polymarket-execution-verification)

Automated trading systems often treat **ORDER FILLED** as the end of the execution lifecycle. A production system usually needs to distinguish:

```text
order accepted → matched → filled → transaction pending → confirmed → settled → position verified
```

**Polymarket Execution Verifier** is Rust infrastructure that tracks those stages explicitly and surfaces incomplete, delayed, or inconsistent execution state.

> **Do not let a trading system assume an execution is complete before the relevant state has been verified.**

```text
Trading Bot
    ↓
Order / Execution
    ↓
Execution Verifier          ← this repository
    ↓
Position Reconciliation
    ↓
Risk / Control Plane
```

**Status:** Working library core (state machine, idempotent events, tests, SQLite persistence, mock gateway). Live Polymarket SDK wiring is read-oriented and feature-gated (`polymarket-sdk`).

---

## Problem

A bot can receive a fill event while transaction confirmation, settlement, and portfolio state are still unknown. Treating `filled = true` as “done” hides operational risk.

## Why FILLED is not enough

| Stage | What it proves |
|-------|----------------|
| FILLED | CLOB/trade observation of matched size |
| TX_PENDING | On-chain settlement may still be in flight |
| CONFIRMED | Transaction evidence observed |
| SETTLED | Settlement stage verified separately |
| POSITION_VERIFIED | Observed portfolio matches expected delta |

The verifier keeps these stages separate.

## Execution lifecycle

Normal path:

```text
INTENDED → SUBMITTED → ACCEPTED → MATCHED → PARTIAL? → FILLED
  → TX_PENDING → CONFIRMED → SETTLED → POSITION_VERIFIED
```

Uncertainty / failure:

```text
REJECTED | CANCELLED | FAILED | RETRYING | UNKNOWN | INCONSISTENT | RECONCILING
```

`PARTIAL` retains `requested_quantity`, `matched_quantity`, and `remaining_quantity` without inferring fill from intent.

## Architecture

```text
src/
  domain/          execution model, states, events, identifiers
  verifier/        transition validator + event engine
  gateway/         traits + mock (+ optional polymarket adapter)
  reconciliation/  recovery + position compare
  repository/      in-memory + SQLite
  telemetry/       structured tracing helpers
```

Design rules implemented in code:

- Execution verification is separate from position reconciliation (traits + modules).
- Observations (`ExecutionEvent`) are separate from derived `VerificationState`.
- Terminal states (`POSITION_VERIFIED`, `INCONSISTENT`, etc.) do not regress on duplicate/stale events.
- Financial quantities use `rust_decimal::Decimal`.

## State machine

Transitions are allow-listed in `verifier/transitions.rs`. Illegal transitions return `VerifierError::IllegalTransition`.

Examples:

- `FILLED → TX_PENDING → CONFIRMED → SETTLED → POSITION_VERIFIED`
- `MATCHED → PARTIAL → FILLED`
- `CONFIRMED → INCONSISTENT` (position mismatch)
- `FILLED → UNKNOWN` (missing evidence — not silently promoted to success)

## Partial fills

Multiple `FillObserved` events aggregate idempotently by `trade_id`:

```text
requested=100, fill 40 → PARTIAL (remaining 60)
+ fill 20 → PARTIAL (remaining 40)
+ fill 40 → FILLED (remaining 0)
```

Duplicate trade IDs are ignored.

## Transaction verification

Fill state does not imply transaction confirmation. The engine supports `TransactionPending`, `TransactionObserved`, and `TransactionConfirmed` as distinct events. Temporary lookup failures map to `UNKNOWN` / `RETRYING`, not automatic `FAILED`.

## Position verification

`reconciliation/position.rs` compares expected vs observed quantities. Mismatch emits `INCONSISTENT` with a structured `PositionMismatch` (expected, observed, difference, execution IDs).

`POSITION_VERIFIED` is not inferred from `FILLED` alone.

## Recovery

`ReconciliationExecutor::recover_execution`:

1. Mark recovery / reconcile
2. Query authoritative order + trades via `ExecutionSource`
3. Query transaction status via `TransactionSource` when a hash exists
4. Compare positions via `PositionSource`
5. Recompute execution + verification state

Reconnecting alone is not treated as recovered; state must be explicitly verified again.

## Persistence

- `InMemoryExecutionRepository` — unit tests
- `SqliteExecutionRepository` — local development (JSON payload column + migration)

Trait: `ExecutionRepository` (`save`, `get`, `list`).

## Tests

```bash
cargo test
```

Coverage includes:

- Valid/invalid transitions and terminal-state protection
- Partial and multi-fill aggregation + duplicate fills
- Transaction pending → confirmed; unknown remains uncertain
- Position match / mismatch / missing
- Event gap + recovery via mock gateway
- Repository round-trip (memory + SQLite)

## Local demo (no credentials)

```bash
cargo run --example basic_verification
```

Prints three scenarios:

- **A:** partial fill, transaction pending
- **B:** partial fill, confirmed, position verified
- **C:** full fill, confirmed, position mismatch → `INCONSISTENT`

## Polymarket adapter

Optional feature `polymarket-sdk` depends on [`polymarket_client_sdk_v2`](https://crates.io/crates/polymarket_client_sdk_v2) (maintained V2 client; not archived `rs-clob-client`).

`gateway/polymarket.rs` is **read-oriented** and documents field mapping. Host applications supply configured SDK clients; this repo does **not** submit live orders.

Enable:

```toml
polymarket-execution-verifier = { path = ".", features = ["polymarket-sdk"] }
```

## Development

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

CI: `.github/workflows/ci.yml`

MSRV: Rust **1.88** (`rust-toolchain.toml`).

## Related projects

- Trading Bot: https://github.com/casatrickdev/polymarket-trading-bot
- Execution Verifier: https://github.com/casatrickdev/polymarket-execution-verifier
- Trading Control Plane: https://github.com/casatrickdev/polymarket-trading-control-plane

## What this is not

Not a trading strategy, arbitrage bot, or order-placement service. Not financial advice.

**Build the strategy. Verify the execution. Trust the state.**
