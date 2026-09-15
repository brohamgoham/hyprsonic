# HyprSonic Lane 1 Phase 0 — completion report

The synthetic multi-venue capital-plan demo was developed on `lane1-phase0` and subsequently committed and pushed to `master` as `e7e5586`, per the owner's request to publish completed phases directly. The complete demonstration ran in **3.706 seconds**, including compilation into a separate target directory with dependencies already cached. It made no runtime network requests and used no accounts or credentials. Timing is one local observation, not a performance guarantee.

## Run

```sh
cd /home/h3cker/grok-bot/hyprsonic
bash scripts/demo.sh
```

For a fresh checkout, first run `cargo fetch --locked` to obtain the three direct dependencies and their build dependencies. The demo subsequently builds and runs offline. Rust 1.88+ is required; validation used the installed Rust 1.100.0 nightly toolchain.

## The failure and alternative

All four plans execute the same proposed action at tick 2. Both scenarios apply the same tick-5 price move. Only the payout's settlement time changes.

From [the recorded wallet failure](../artifacts/wallet-failure.log):

```text
#006 t=1 [POLY_CHALLENGE_DELAY] $3000.00 pending proceeds delayed from t=3 to t=9; funding contribution before settlement = $0.00
#014 t=5 [HL_MARGIN_UNMET] Aggregate marked equity $14099.00 is positive; Hyperliquid has $2500.00 against local maintenance $4000.00; shortfall $1500.00. External pending/pledged capital is not margin in time
RESULT=FAIL (selected horizon only)
```

The standalone failed replay exits **2**, including when producing JSON. This is checked by the demonstration script and CLI integration test.

| Plan | On-time payout | Delayed payout + same price move | Delayed cost | Delayed local buffer |
|---|---|---|---:|---:|
| Wallet | Survives | Fails | $1 | -$1,500 |
| Withdraw settled funds | Survives | Fails | $2 | -$800 |
| Reduce existing position, then wallet | Survives | Survives | $31 | $470 |
| Synthetic financing quote | Survives | Survives | $36 | $180 |

Against the cheapest successful on-time plan ($2), the delayed-scenario alternatives cost an additional **$29** and **$34** respectively. Reduction lowers existing exposure, so the comparison is not a claim of equal strategy P&L. Financing leaves **$3,216 owed**, keeps collateral pledged, and has a maturity beyond the horizon. Borrowing is never counted as new equity.

The demo also breaks synthetic financing after the pledge. The ledger retains both pledged lots, records zero disbursed credit and zero loan liability, and cancels the dependent action. It does not roll back the pledge as though the steps were atomic.

## Deliverables

| Requirement | Evidence |
|---|---|
| Three synthetic venue stubs and one proposed action | [Fixture](../fixtures/lane1.json), [model](../src/model.rs) |
| Settled, withdrawable, unrealized, pending, pledged/reserved, delayed/blocked | Identified lots and distinct views in each ledger snapshot; overlapping totals explicitly documented |
| Four plans, costs, dependencies, partial failures | [Replay engine](../src/engine.rs), [plan table](../README.md#four-funding-plans) |
| Price move with delayed payout and feasible alternative | [Demo output](../artifacts/phase0-demo.log) |
| Ordered event evidence with exact fixture | [JSON ledger](../artifacts/wallet-failure.json) |
| Under-five-minute script | [Script](../scripts/demo.sh), [measured timing](../artifacts/demo-timing.json) |
| Required tests plus accounting and failure boundaries | [16 integration tests](../tests/phase0.rs) |
| Synthetic scope and commercial-access limitation | [README](../README.md), report fixture flags and lifecycle events |

## Verification

- `cargo test --offline --locked`: **16 passed**.
- `cargo clippy --offline --locked --all-targets -- -D warnings`: **passed**.
- `cargo fmt --all -- --check`: **passed**.
- Bash syntax and ShellCheck for `scripts/demo.sh`: **passed**.
- Full demo script: **exit 0**, **3.706 seconds** including compilation.
- Infeasible CLI replay: **exit 2**, valid JSON with `feasible: false` and the causal ledger.

Coverage includes double pledge, attempts to withdraw pledged funds, delayed settlement, positive aggregate equity with unmet local margin, debt and fee conservation, a one-cent boundary breach, a reduction fee preceding its fill, all four injected plan failures, transfer latency, expired quotes, Kalshi lifecycle separation, late recovery that cannot erase a breach, invalid input, and deterministic output.

## Limits and decision gate

Every price, fee, transfer duration, lending term, margin rule, and fill is synthetic. Venue names label rule stubs. The CLI has no live market SDK, network client, execution path, or Kalshi adapter. Kalshi commercial access remains explicitly unmet. The old stream-reader entry point and SDK dependencies were removed.

Survival is limited to the selected horizon. Pending claims use fixture marks; liquidation execution, loan repayment at maturity, actual counterparty arrangements, and market-calibrated risk are outside Phase 0. The ledger continues diagnostically after a breach without simulating liquidation, and the result remains failed.

This delivers the agreed technical demonstration, not validation that customers will pay. **No interviews or outreach until Mo greenlights the demo.** If users would not buy the planning/capital-state workflow without live credit, stop Lane 1 rather than treating technical completion as commercial proof. Cantina remains a separate lane.

Published to [brohamgoham/hyprsonic](https://github.com/brohamgoham/hyprsonic/commit/e7e5586) on `master`. No PR was needed. The [product decision](product.md) and [architecture contract](architecture.md) define the real-data phases that follow; they do not change this report's synthetic scope.
