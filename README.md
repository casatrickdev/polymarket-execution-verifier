# Polymarket Execution Verifier

> Verify Polymarket trade state across order, fill, transaction, and settlement stages.

GitHub: [polymarket execution verifier](https://github.com/casatrickdev/polymarket-execution-verifier)
Automated trading systems often treat:

```text
ORDER FILLED
```

as the end of the execution lifecycle.

It isn't always.

A production trading system may need to distinguish between:

```text
order accepted
→ matched
→ filled
→ transaction pending
→ transaction confirmed
→ settlement verified
→ position updated
```

**Polymarket Execution Verifier** is an experimental infrastructure layer for tracking those states explicitly and identifying when execution information is incomplete, delayed, or inconsistent.

The goal is simple:

> **Don't let a trading system assume an execution is complete before the relevant state has been verified.**

**Status:** Early development

---

## Why this exists

A trading bot can receive a successful order response and still have uncertainty about what happened afterward.

For example:

```text
CLOB
  ↓
ORDER FILLED
  ↓
transaction not yet confirmed
  ↓
local system says "done"
```

Or:

```text
trade event received
       ↓
transaction lookup delayed
       ↓
position state not updated
       ↓
strategy sees inconsistent exposure
```

These are operational problems.

They become especially important when a trading system needs to coordinate:

* execution
* positions
* risk
* reconciliation
* recovery

This project explores a dedicated verification layer for that part of the stack.

---

# Core idea

Instead of treating execution as one boolean:

```text
filled = true
```

model it as an explicit lifecycle:

```text
INTENDED
   ↓
SUBMITTED
   ↓
ACCEPTED
   ↓
MATCHED
   ↓
FILLED
   ↓
TX_PENDING
   ↓
CONFIRMED
   ↓
SETTLED
```

With alternative paths:

```text
SUBMITTED → REJECTED
SUBMITTED → CANCELLED
MATCHED   → RETRYING
FILLED    → UNKNOWN
```

The system should be able to represent uncertainty instead of forcing every execution into `success` or `failure`.

---

# Architecture

```text
                        POLYMARKET
                            │
             ┌──────────────┼──────────────┐
             │              │              │
             ▼              ▼              ▼
          CLOB           Trades         Chain Data
             │              │              │
             └──────────────┼──────────────┘
                            ▼
                  ┌──────────────────┐
                  │ Execution        │
                  │ Verifier         │
                  └────────┬─────────┘
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
         Order State   Trade State   Chain State
             │             │             │
             └─────────────┼─────────────┘
                           ▼
                  ┌──────────────────┐
                  │ State Resolver   │
                  └────────┬─────────┘
                           │
                  ┌────────▼─────────┐
                  │ Verification     │
                  │ Result           │
                  └──────────────────┘
```

The verifier sits between raw execution information and the trading system's trusted state.

---

# What it should answer

For a given execution:

### What was requested?

```text
Order:
  market
  side
  price
  size
```

### What was matched?

```text
Trade:
  trade_id
  matched_size
  execution_price
```

### What happened on-chain?

```text
Transaction:
  hash
  status
  confirmation
```

### What state can we safely trust?

```text
VERIFIED
PENDING
FAILED
INCONSISTENT
UNKNOWN
```

---

# Execution state model

The first version uses explicit states rather than hidden assumptions.

```text
                ┌─────────────┐
                │   INTENDED  │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │  SUBMITTED  │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │   ACCEPTED  │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │   MATCHED   │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │    FILLED   │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │  TX_PENDING │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │  CONFIRMED  │
                └──────┬──────┘
                       ▼
                ┌─────────────┐
                │   SETTLED   │
                └─────────────┘
```

Possible failure or uncertainty states:

```text
REJECTED
CANCELLED
FAILED
RETRYING
UNKNOWN
INCONSISTENT
```

---

# Why `UNKNOWN` matters

A trading system should not convert uncertainty into success.

For example:

```text
CLOB:
FILLED

Chain:
confirmation unavailable
```

The correct result may be:

```text
EXECUTION = UNKNOWN
```

not:

```text
EXECUTION = CONFIRMED
```

That distinction allows an upstream risk engine to decide what happens next.

---

# Verification flow

```text
Order / Trade Event
        ↓
Normalize
        ↓
Load known state
        ↓
Resolve available execution data
        ↓
Check transaction state
        ↓
Compare related states
        ↓
Produce verification result
```

Example:

```text
CLOB:
FILLED

Trade:
40 contracts

Transaction:
PENDING

Position:
unchanged

Result:
IN_PROGRESS
```

Another example:

```text
CLOB:
FILLED

Trade:
40 contracts

Transaction:
CONFIRMED

Position:
+40

Result:
VERIFIED
```

---

# Reconciliation

The verifier is designed to work alongside a trading-system reconciler.

For example:

```text
Local Execution
      │
      ▼
Expected State
      │
      ├───────────────┐
      │               │
      ▼               ▼
 CLOB / Trade     Chain State
      │               │
      └───────┬───────┘
              ▼
         Compare
              │
        ┌─────┴─────┐
        ▼           ▼
     MATCH        MISMATCH
        │           │
        ▼           ▼
    VERIFIED      REVIEW
```

A mismatch should be explicit and observable.

Example:

```text
EXECUTION MISMATCH

order_id: 123

CLOB:
  FILLED

Trade:
  40

Chain:
  NOT_CONFIRMED

Position:
  0

Status:
  INCONSISTENT
```

---

# Failure scenarios

The project should eventually cover:

| Scenario               | Expected result         |
| ---------------------- | ----------------------- |
| Normal fill            | `VERIFIED`              |
| Delayed confirmation   | `PENDING`               |
| Failed transaction     | `FAILED`                |
| Missing execution data | `UNKNOWN`               |
| Position mismatch      | `INCONSISTENT`          |
| Duplicate event        | Ignore duplicate safely |
| WebSocket disconnect   | Reconcile               |
| Process restart        | Reload and verify       |
| API unavailable        | Preserve uncertainty    |
| Chain lookup delayed   | Keep execution pending  |

---

# Example lifecycle

A healthy execution:

```text
SIGNAL
  ↓
ORDER SUBMITTED
  ↓
ORDER ACCEPTED
  ↓
TRADE MATCHED
  ↓
FILL RECEIVED
  ↓
TRANSACTION CONFIRMED
  ↓
POSITION VERIFIED
  ↓
EXECUTION VERIFIED
```

A degraded execution:

```text
SIGNAL
  ↓
ORDER SUBMITTED
  ↓
ORDER ACCEPTED
  ↓
TRADE MATCHED
  ↓
WEBSOCKET DISCONNECT
  ↓
LOCAL STATE UNCERTAIN
  ↓
RECONCILIATION
  ↓
VERIFY
```

---

# Why this belongs outside the strategy

The strategy should decide:

> **What should I trade?**

The execution layer should decide:

> **How should I place it?**

The verifier should decide:

> **What do I actually know about the execution?**

Keeping these responsibilities separate makes the verification layer reusable across strategies.

For example:

```text
Momentum
   │
Arbitrage
   │
Market Making
   │
Copy Trading
   │
   ▼
Execution Verifier
   │
   ▼
Polymarket
```

---

# Polymarket integration

This project is designed to consume execution information from the current Polymarket stack rather than replace it.

The maintained Polymarket Rust client V2 provides typed CLOB APIs plus WebSocket streams for orderbook, price, authenticated order, and trade events. It also exposes Data/Gamma integrations and CTF functionality.

The older `rs-clob-client` repository is archived and should not be used for new integrations; Polymarket directs developers to the V2 client.

The current V2 client changelog also documents an asynchronous execution flow in which matched orders may carry `tradeIDs` and transaction hashes can be resolved afterward.

That makes explicit execution-state modeling useful for this project.

---

# Project structure

```text
polymarket-execution-verifier/
│
├── src/
│   ├── ingestion/
│   ├── execution/
│   ├── state/
│   ├── verification/
│   ├── reconciliation/
│   └── errors/
│
├── examples/
│   ├── verify_fill/
│   ├── pending_execution/
│   └── state_mismatch/
│
├── tests/
│   ├── lifecycle/
│   ├── verification/
│   ├── reconciliation/
│   └── failure_cases/
│
├── docs/
│   ├── architecture.md
│   ├── state-machine.md
│   └── failure-model.md
│
├── Cargo.toml
├── Cargo.lock
└── README.md
```

---

# Development roadmap

## Phase 1 - Execution model

* [ ] Order state model
* [ ] Trade state model
* [ ] Transaction state model
* [ ] Position state model
* [ ] Explicit lifecycle transitions

## Phase 2 - Verification

* [ ] Match order and trade records
* [ ] Track execution status
* [ ] Resolve transaction state
* [ ] Detect incomplete execution
* [ ] Produce verification result

## Phase 3 - Reconciliation

* [ ] Detect mismatches
* [ ] Recover after disconnect
* [ ] Recover after restart
* [ ] Re-check pending executions
* [ ] Persist verification history

## Phase 4 - Risk integration

* [ ] Expose verification state to risk engine
* [ ] Block trading on critical mismatch
* [ ] Configurable recovery policy
* [ ] Execution uncertainty thresholds

## Phase 5 - Observability

* [ ] Structured logs
* [ ] Execution timeline
* [ ] Verification metrics
* [ ] Alerts
* [ ] Incident replay

---

# Example API

A future interface could look like:

```rust
let result = verifier.verify(order_id).await?;

println!("status: {:?}", result.status);
println!("matched: {}", result.matched_size);
println!("confirmed: {}", result.confirmed);
```

Possible result:

```json
{
  "order_id": "123",
  "status": "VERIFIED",
  "matched_size": "40",
  "transaction_confirmed": true,
  "position_verified": true
}
```

Another result:

```json
{
  "order_id": "123",
  "status": "UNKNOWN",
  "matched_size": "40",
  "transaction_confirmed": false,
  "position_verified": false,
  "reason": "execution_state_incomplete"
}
```

---

# Design principles

### Explicit state

Execution stages should be represented directly.

### Don't hide uncertainty

Unknown should remain unknown until enough evidence exists.

### Verify before trusting

A local event should not automatically become trusted portfolio state.

### Fail safely

Critical execution inconsistencies should be able to block additional risk.

### Strategy independent

Verification should work regardless of the strategy generating the order.

### Replayable

Execution history should eventually be replayable for debugging and incident analysis.

---

# What this project is not

This is **not**:

* a profitable trading strategy
* an arbitrage bot
* a copy-trading service
* a prediction engine
* a guarantee of successful settlement
* financial advice

It is an infrastructure experiment focused on execution-state verification.

---

# Intended users

This project is intended for developers building:

* Polymarket trading bots
* arbitrage systems
* market-making systems
* copy-trading systems
* automated execution engines
* quantitative trading infrastructure

The same concepts can also be adapted to other event-driven trading systems where execution state can arrive through multiple channels.

---

# Why this matters

A trading system can know:

```text
“I sent the order.”
```

It can know:

```text
“The order was matched.”
```

It may even know:

```text
“The trade event was received.”
```

But the system still needs to answer:

> **What state can I safely trust right now?**

That's the problem this project is designed to explore.

---

# Roadmap direction

The longer-term goal is to connect execution verification with the broader trading-system control plane:

```text
Strategy
   ↓
Execution
   ↓
Execution Verifier
   ↓
Reconciliation
   ↓
Risk
   ↓
Monitoring
   ↓
Trading Control
```

That creates a reusable infrastructure layer for automated Polymarket systems.

---

# Status

**Early development.**

The initial implementation focuses on explicit execution states, verification, reconciliation, and failure handling.

No profitability claims are made.

---

# Contributing

Contributions and architecture feedback are welcome, especially around:

* event-driven state machines
* execution verification
* reconciliation
* WebSocket recovery
* trading-system reliability
* Rust async architecture
* observability

---

# Keywords

`Polymarket` · `Polymarket API` · `Polymarket CLOB` · `Polymarket WebSocket` · `Polymarket trading bot` · `Polymarket execution` · `Polymarket settlement` · `Polymarket trading system` · `execution verification` · `order reconciliation` · `algorithmic trading` · `trading infrastructure` · `Rust` · `real-time systems`

---

## About Casatrick

Casatrick builds trading, data, and automation systems for Polymarket, with a focus on execution, real-time infrastructure, reliability, risk, and production engineering.

This project explores one of the less visible problems in automated trading:

**knowing what actually happened after an order was sent.**

## Explore the project

The implementation is evolving around execution verification, reconciliation, and failure handling for automated Polymarket trading systems.

If you're building a Polymarket trading system and dealing with execution state, reconciliation, or reliability problems, feel free to open an issue or start a discussion.

**Build the strategy. Verify the execution. Trust the state.**
