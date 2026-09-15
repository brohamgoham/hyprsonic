<p align="center">
  <img src="assets/hyprsonic-header.png" alt="HyprSonic — Fund the next move." width="100%" />
</p>

<p align="center">
  <strong>Capital planning across trading accounts.</strong><br />
  Know what can move. Understand what must wait. Keep the next move funded.
</p>

<p align="center">
  <a href="docs/observations.md">Observe accounts</a> ·
  <a href="#try-the-demo">Try the demo</a> ·
  <a href="docs/roadmap.md">Roadmap</a> ·
  <a href="docs/product.md">Product</a> ·
  <a href="docs/architecture.md">Architecture</a>
</p>

---

## Your portfolio has value. Can it fund your next move?

Money can be settled in one account, supporting margin in another, or waiting on an outcome to resolve. A healthy portfolio total can hide a funding shortfall at the account that needs it.

**HyprSonic is being built to turn those constraints into a funding plan.** Connect Hyperliquid, Polymarket predictions, and a wallet. Specify where you need capital, by when, and which reserves to preserve. Compare the available routes and follow their dependencies through to destination credit.

> **Development status:** real read-only account observations and the synthetic CLI demo are implemented. Public-reference live checks pass; the own-account acceptance check is pending configuration. Capital reconciliation, funding plans from live data, and the workspace are next. No signing or money movement is implemented.

## Connect real accounts

Setup and demo instructions are for the owner and users with written permission. See [License](#license).

The `observe` command reads Hyperliquid, Polymarket positions, and EVM wallet balances through ports and adapters. It records account mode, exact amounts, source evidence and missing coverage. Every report keeps funding eligibility **undetermined** until reconciliation and venue rules are implemented.

```sh
umask 077
mkdir -p .local
cp -n config/accounts.example.json .local/accounts.json
# Edit the copied config with your public account addresses.
# Set the configured RPC environment variables for EVM wallet reads.
cargo run --locked -- observe --config .local/accounts.json
```

**[Account setup, RPC configuration and coverage →](docs/observations.md)**

The command writes private evidence under `.local/observations/`. A failed read exits `3` and reports the gap; it does not become a zero balance. Use `--json` for all observations. The [Phase 1 report](docs/phase1-report.md) separates live-reference evidence from the pending own-wallet check.

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
| **1** | Real account connections and core boundaries | Implemented; own-account check pending |
| **2** | Reconciled capital states and supported account rules | Planned |
| **3** | One complete funding request with verified routes | Planned |
| **4** | Continuous observation and a minimal workspace | Planned |
| **5** | Own-wallet trial with actual destination reconciliation | Planned |
| **6** | Repeated use, fixes, and an internal release | Planned |
| **7** | Explicit decision on an external paid pilot | Gated |

Each phase has deliverables, dependencies, failure checks, and an acceptance gate in the **[full delivery plan](docs/roadmap.md)**. The [own-wallet runbook](docs/own-wallet-testing.md) defines how we'll gather real evidence without confusing replayed failures with live events.

## Built around a capital core

The workspace now has a pure Rust observation core and an application with account-reader, clock and evidence-store ports. Market feeds, route quotes, receipt tracking and planning will extend that boundary in subsequent phases.

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

The diagram shows the full target workflow. **Observe and its core/adapter boundary are implemented**; Plan, Watch and the workspace UI remain planned. The Phase 0 engine stays a separate synthetic model, with no live observations fed into its arbitrary rules.

Fast recomputation and fresh evidence are separate requirements. We'll measure upstream age, processing delay, decision updates, and display latency independently. [Read the architecture contract →](docs/architecture.md)

## Development

From the repository root, after fetching dependencies:

```sh
cargo test --workspace --offline --locked
cargo clippy --workspace --offline --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

The suite includes the original 16 Phase 0 cases plus core, adapter and transport checks: exact decimals, pagination gaps, identity/mode errors, token validation, stale data, reorgs, evidence privacy and HTTP limits. Tests use deterministic inputs and local loopback servers; external network checks are opt-in through `observe`. See [current coverage](docs/observations.md#coverage).

| Read next | Purpose |
|---|---|
| [Product decision](docs/product.md) | The user, recurring job, and commercial hypothesis |
| [Observe real accounts](docs/observations.md) | Configuration, endpoint coverage, private evidence, and exit codes |
| [Delivery plan](docs/roadmap.md) | Full phases and completion gates |
| [Architecture](docs/architecture.md) | Domain model, ports, adapters, and evidence semantics |
| [Own-wallet testing](docs/own-wallet-testing.md) | Internal trial sequence and decision journal |
| [Demo reference](docs/demo.md) | Commands, fixture assumptions, and accounting details |

## License

**Proprietary. Copyright © 2026 Mo Maali (brohamgoham). All rights reserved.**

The source is visible for inspection; HyprSonic is not open source. Running, modifying, redistributing, or offering it as a service requires prior written permission, subject to the exceptions in [LICENSE](LICENSE). GitHub's public-repository viewing and forking rights still apply. For permission or commercial licensing, use the contact details on [Mo's profile](https://github.com/brohamgoham).

Third-party dependencies, characters, and trademarks retain their respective rights and licenses.

<sub>Header artwork: generated for this project. Asset and generation notes live in <a href="assets/README.md">assets/</a>.</sub>
