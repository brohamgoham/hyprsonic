<p align="center">
  <img src="assets/hyprsonic-header.png" alt="HyprSonic — Fund the next move." width="100%" />
</p>

<p align="center">
  <strong>Capital planning across trading accounts.</strong><br />
  Know what can move. Understand what must wait. Keep the next move funded.
</p>

<p align="center">
  <a href="#try-the-demo">Try the demo</a> ·
  <a href="docs/roadmap.md">Roadmap</a> ·
  <a href="docs/product.md">Product</a> ·
  <a href="docs/architecture.md">Architecture</a>
</p>

---

## Your portfolio has value. Can it fund your next move?

Money can be settled in one account, supporting margin in another, or waiting on an outcome to resolve. A healthy portfolio total can hide a funding shortfall at the account that needs it.

**HyprSonic is being built to turn those constraints into a funding plan.** Connect Hyperliquid, Polymarket predictions, and a wallet. Specify where you need capital, by when, and which reserves to preserve. Compare the available routes and follow their dependencies through to destination credit.

> **Development status:** the synthetic CLI demo is shipped. Live adapters, the funding workspace, and own-wallet trials are planned. The current executable uses no accounts, keys, network requests, or real funds.

## One request. The whole funding path.

The workflow we're building:

> “Fund this Hyperliquid account before my deadline. Preserve my cash reserve elsewhere and my margin buffer under this price move.”

| Step | What you get |
|---|---|
| **Connect** | Actual account identities, collateral pools, holdings, and visible data gaps |
| **Plan** | Supported funding alternatives with costs, timing assumptions, and remaining buffers |
| **Follow** | A timeline of prerequisites, transfers, confirmations, and destination credit |
| **Recheck** | A revised answer when prices, account activity, or settlement timing change |
| **Explain** | The observations and rules behind every amount, blocker, and decision |

You choose and perform money movements through your native wallet or venue. The planned product observes their progress. Financing contributes only when a real, supported provider offer exists.

## Capital has states

| State | The distinction that matters |
|---|---|
| **Settled** | Funds have been credited; some may still be committed |
| **Withdrawable** | Funds are available under the account's withdrawal constraints |
| **Unrealized** | Marked gains or losses, with venue-specific margin treatment |
| **Pending proceeds** | Claims awaiting settlement or redemption |
| **Pledged / reserved** | Capital already committed to an obligation or plan |
| **Delayed / blocked** | Capital affected by a delay, restriction, or incomplete transfer |

These are overlapping views of identified holdings, **not six balances to add together**. Unknown data remains unknown. Source confirmation and destination credit are separate events.

## Try the demo

Requires **Rust 1.88+** and Cargo.

```sh
git clone https://github.com/brohamgoham/hyprsonic.git
cd hyprsonic
cargo fetch --locked
bash scripts/demo.sh
```

The demo runs offline once dependencies are cached. It compares wallet funding, available withdrawals, position reduction, and a **synthetic** financing quote across on-time and delayed-payout scenarios.

The failure it exposes:

```text
[HL_MARGIN_UNMET] Aggregate marked equity $14099.00 is positive;
Hyperliquid has $2500.00 against local maintenance $4000.00;
shortfall $1500.00.
```

Under that same synthetic delay and price move, wallet and withdrawal plans fail. Reducing existing exposure survives at a modeled $31 cost. Synthetic financing survives at $36, leaving $3,216 owed. Survival covers only the selected scenario horizon.

The script saves text and JSON evidence. An infeasible standalone replay exits **2**; the demo script succeeds after verifying that failure. The recorded run took **3.706 seconds**, including compilation with cached dependencies; it is not a live-system latency benchmark.

[Demo reference](docs/demo.md) · [Recorded output](artifacts/phase0-demo.log) · [Failure ledger](artifacts/wallet-failure.log) · [Phase 0 report](docs/phase0-report.md)

## Build → use → tune

Our first release target is a workflow we use with **our own wallets** before offering it to anyone else.

| Phase | Milestone | Status |
|---|---|---|
| **0** | Synthetic capital-plan proof and failure ledger | Shipped |
| **1** | Real account connections and core boundaries | Next |
| **2** | Reconciled capital states and supported account rules | Planned |
| **3** | One complete funding request with verified routes | Planned |
| **4** | Continuous observation and a minimal workspace | Planned |
| **5** | Own-wallet trial with actual destination reconciliation | Planned |
| **6** | Repeated use, fixes, and an internal release | Planned |
| **7** | Explicit decision on an external paid pilot | Gated |

Each phase has deliverables, dependencies, failure checks, and an acceptance gate in the **[full delivery plan](docs/roadmap.md)**. The [own-wallet runbook](docs/own-wallet-testing.md) defines how we'll gather real evidence without confusing replayed failures with live events.

## Built around a capital core

The next implementation uses a pure Rust core behind ports and adapters. Account readers, market feeds, route quotes, receipts, and storage supply evidence to the same planning use cases.

```text
                    CLI / workspace
                           │
                Observe · Plan · Watch · Explain
                           │
                    Pure capital core
                           │
                   Ports and adapters
                           │
       Hyperliquid · Polymarket · wallets · routes
```

This is the **planned architecture**. The shipped Phase 0 engine is still a standalone synthetic model. The next phase develops the core alongside real adapters; live observations will not be fed into arbitrary fixture rules.

Fast recomputation and fresh evidence are separate requirements. We'll measure upstream age, processing delay, decision updates, and display latency independently. [Read the architecture contract →](docs/architecture.md)

## Development

From the repository root, after fetching dependencies:

```sh
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

Phase 0 has **16 passing integration tests**, including double pledging, settlement delay, local margin failure despite positive aggregate equity, debt accounting, and non-atomic failures. Account support and release status will be documented as live phases ship.

| Read next | Purpose |
|---|---|
| [Product decision](docs/product.md) | The user, recurring job, and commercial hypothesis |
| [Delivery plan](docs/roadmap.md) | Full phases and completion gates |
| [Architecture](docs/architecture.md) | Domain model, ports, adapters, and evidence semantics |
| [Own-wallet testing](docs/own-wallet-testing.md) | Internal trial sequence and decision journal |
| [Demo reference](docs/demo.md) | Commands, fixture assumptions, and accounting details |

<sub>Header artwork: generated for this project. Asset and generation notes live in <a href="assets/README.md">assets/</a>.</sub>
