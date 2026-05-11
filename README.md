# cc_order_book_sim

A limit order book simulator in Rust, verified against published NASDAQ data.

## Headline results

- **548,307** real NASDAQ messages replayed across AAPL + GOOG (2012-06-21).
- **0** book-state discrepancies versus LOBSTER's published snapshots at depth 9.
- **~650,000 messages/sec** single-threaded, including CSV parsing.
- **4** property-based invariants, each checked against 256 randomized scenarios.

## What it does

Maintains a price-time priority limit order book. Supports limit + market orders, cancel-by-id, partial fills, and emits a typed event stream (`Accepted`, `Rejected`, `Canceled`, `Trade`).

Two verification paths are wired in:

1. **Property tests** drive a synthetic random-walk feed through the matching engine and assert invariants after every step (book never crosses, quantity is conserved, etc.).
2. **LOBSTER replay** reads a full trading day of real NASDAQ message data, applies each message to a price-level book, and compares the resulting top-of-book state against LOBSTER's published snapshot for that step.

## Architecture

Two book types, each with a clear purpose:

| | `OrderBook` (`src/book.rs`) | `LobsterBook` (`src/replay.rs`) |
|---|---|---|
| Granularity | order-level (every individual order) | price-level (aggregate size per price) |
| Has matching engine | yes | no — applies pre-decided events |
| Driven by | synthetic feed | real LOBSTER messages |
| Verified by | property tests | per-step snapshot diff |

The split is honest about what each dataset supports. The synthetic feed exercises the matching engine; the LOBSTER stream verifies that we interpret real exchange events correctly. Trying to drive the matching engine with LOBSTER messages would require synthesizing fake taker orders, which doesn't strengthen the verification.

### Key design choices

- **Prices as fixed-point integers** (`Price(i64)`), never floats. Required for exact equality and `Ord`.
- **Two `BTreeMap<Price, PriceLevel>`** for bids and asks — gives `O(log n)` best-price access and ordered iteration when sweeping multiple levels.
- **`HashMap<OrderId, (Side, Price)>` index** — cancel-by-id in `O(log n + level_size)` instead of scanning the whole book.
- **`VecDeque<Order>` per price level** — FIFO time priority with `O(1)` front/back ops.

## Running it

```bash
# Property tests
cargo test --release

# LOBSTER replay (depth-10 sample)
cargo run --release -- replay \
  data/LOBSTER_SampleFile_AAPL_2012-06-21_10/AAPL_2012-06-21_34200000_57600000_message_10.csv \
  data/LOBSTER_SampleFile_AAPL_2012-06-21_10/AAPL_2012-06-21_34200000_57600000_orderbook_10.csv \
  10
```

Sample data comes from [lobsterdata.com](https://lobsterdata.com/info/DataSamples.php) — free academic samples for AAPL, GOOG, MSFT, INTC, AMZN at depth 1/5/10.

## Verification depth

LOBSTER's depth-10 sample tells us the top 10 levels of each side after each message. When a top-of-book level is deleted, a deeper level (depth 11+) becomes visible — but we have no record of what was there, since it was outside the sample's depth window.

To make the comparison sound, the replay verifies the top **9** levels strictly. This isolates "is the matching logic correct" from "do we have visibility into deeper book state" — the former is what the engine controls, the latter is a property of the input dataset.

A real deployment with full depth-50+ data would verify all visible levels.

## Layout

```
src/
  book.rs      OrderBook + matching engine
  events.rs    BookEvent: Accepted / Rejected / Canceled / Trade
  feed.rs      random walk generator + LOBSTER CSV parser
  level.rs     PriceLevel (VecDeque<Order> + cached total_qty)
  order.rs     Order, Side, OrderType, Price (i64 ticks), OrderId
  replay.rs    LobsterBook + per-step snapshot verification
  main.rs      CLI: `replay <messages.csv> <orderbook.csv> <depth>`
  lib.rs
tests/
  properties.rs   4 proptest invariants
data/
  LOBSTER_SampleFile_AAPL_2012-06-21_10/
  LOBSTER_SampleFile_GOOG_2012-06-21_10/
```
