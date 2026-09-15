# HyprSonic — Lane 1, Phase 0

**An offline, synthetic multi-venue capital-plan demo.** One question: can this proposed portfolio change be funded and maintained under the selected scenarios?

The portfolio has positive aggregate marked equity, yet Hyperliquid cannot meet its local margin requirement when a Polymarket payout is delayed. The demo compares funding alternatives, including their costs and consequences. It is a product experiment, not a trading system or evidence of customer demand.

This mission supersedes the old SUI feed reader and previous product briefs. The live SDKs and subscriptions have been removed. No terminal, book checker, session-cost tool, live execution, live credit, Kalshi adapter, outreach, or builder fees are included. Cantina is a separate lane.

## Run the demo

Requires Rust 1.88+ and Cargo. From this directory:

```sh
# On a fresh machine, fetch build dependencies once:
cargo fetch --locked

# The demonstration itself uses no network, keys, accounts, or real money:
bash scripts/demo.sh
```

The script builds the small CLI, prints the comparison and failure, and saves the full ledger. It checks that the infeasible wallet replay exits **2**. Once dependencies are cached, compilation and replay should fit comfortably within five minutes; the script prints elapsed time. See the measured run in [the Phase 0 report](docs/phase0-report.md).

Read [the demo output](artifacts/phase0-demo.log), [the full failure log](artifacts/wallet-failure.log), or [the JSON event ledger](artifacts/wallet-failure.json).

Individual replays:

```sh
cargo run --offline --locked -- replay --scenario on-time --plan wallet
cargo run --offline --locked -- replay --scenario delayed --plan reduce
cargo run --offline --locked -- replay --scenario delayed --plan finance --json

# These intentionally return exit status 2:
cargo run --offline --locked -- replay --scenario delayed --plan wallet
cargo run --offline --locked -- replay --scenario delayed --plan finance --break-step
```

Commands return **0** for a feasible plan/successful demo, **2** for an infeasible plan, and **1** for invalid input or an internal accounting error. `--break-step` works with all four plans. `--fixture PATH` accepts another fixture with the same schema; unknown fields, non-synthetic inputs, negative amounts, out-of-bound values, and invalid lifecycle ordering are rejected.

## The fixture and proposed action

All quantities in [fixtures/lane1.json](fixtures/lane1.json) are integer **synthetic USD cents**. One tick is an abstract step, not a second or a promise about venue latency. There are no exchange-rate conversions or real prices.

The proposed action adds the same synthetic Hyperliquid position at tick 2. It requires $5,500 local initial margin, with $4,000 subsequent maintenance. Existing settled collateral starts at $4,000. Initial and maintenance checks are distinct.

Both scenarios apply the **same** price move at tick 5: the existing position loses $2,000, and the proposed position loses $1,000. The only scenario difference is the Polymarket payout: confirmed at tick 3 in `on-time`, challenged at tick 1 and postponed to tick 9 in `delayed`. A transfer takes one additional tick. All plans conditionally sweep that payout once it actually settles.

Kalshi closes at tick 1, determines at tick 3, and settles at tick 8. Neither close nor determination releases its $5,000 pending claim. The six-tick default horizon ends before that settlement. **Kalshi commercial access is explicitly unmet**, including in saved results. Even an extended synthetic replay does not transfer Kalshi proceeds: there is no adapter.

## Capital states are not one cash number

Every ledger event includes individual identified lots and separate views:

| View | Meaning in this model |
|---|---|
| Settled | Withdrawable, settled-but-locally-locked, and reserved/pledged funds |
| Withdrawable | The free subset of settled funds usable by a supported transfer step |
| Unrealized | Signed mark P&L, not a cash payout; the HL stub includes it in local margin |
| Pending proceeds | Unsettled claims, including delayed claims; valued at fixture marks only |
| Pledged/reserved | Settled funds already committed to an identified obligation |
| Delayed/blocked | Challenged claims, transfers in flight, and failed transfers awaiting recovery |

**These columns overlap and must not be summed.** For example, a delayed $3,000 claim is included in both pending and delayed views. A pledged lot remains part of settled assets, but its withdrawable amount is zero. In-flight funds retain an identified lot at `InTransit` and do not count as received Hyperliquid collateral.

Aggregate marked equity is computed once from unique lots plus unrealized P&L minus loan liabilities. Pending claims use an explicitly optimistic fixture mark, not guaranteed recovery value. Every checkpoint verifies equity conservation against costs and modeled P&L. Borrowing creates an equal liability; it cannot manufacture equity. The initially pledged wallet lot backs an existing non-loan commitment, so no existing loan liability is assumed.

## Four funding plans

Each plan starts from an independent copy of the same portfolio. Plans are alternatives, not four simultaneous claims on the same collateral.

| Plan | Steps and costs | Dependency / failure behavior |
|---|---|---|
| `wallet` | Reserve $1,500 plus $1 fee; dispatch; arrive on HL one tick later | Funds cannot be reused after reservation; broken dispatch leaves them blocked. Without the payout, price stress creates a $1,500 local shortfall. |
| `withdraw` | Withdraw $2,200 from the already-settled Polymarket lot; pay $2; arrive one tick later | Cannot withdraw pending or pledged claims; dispatch can fail. Delayed payout leaves an $800 HL shortfall after stress. |
| `reduce` | Pay $30; confirm synthetic reduction of existing exposure to 50%; lower margin requirements by $1,000; then fund via the wallet route | No relief before fill confirmation. Failed fill leaves the fee spent and exposure unchanged. This changes the old position, not the proposed action. |
| `finance` | Pledge $4,100 of free wallet/Polymarket lots; synthetic quote disburses $3,200 less $20 fee at tick 1; accrue $16 horizon interest | Exclusive collateral, 80% synthetic limit, and expiry checked. Failed funding leaves pledges in place and creates no loan. Principal plus interest of $3,216 remains owed. |

Every plan also pays $1 **if** the conditional Polymarket payout sweep occurs. Costs mean modeled transaction fees and financing interest; they are not total strategy P&L. Different exposure makes a reduction a tradeoff, not free capital efficiency. Quote terms, fills, maintenance relief, and loss amounts are all arbitrary fixture assumptions, not venue or lender parameters.

The synthetic loan matures at tick 10, outside the six-tick horizon. The result retains its full liability and checks its collateral limit, but does not model maturity repayment, liquidation fills, or real underwriting. **SURVIVES means only that the proposed action executed and no modeled constraint was breached during the selected horizon.** It is not a claim of safety after that horizon.

## Explainability and failure semantics

The JSON report embeds the exact input fixture, selected plan/scenario, injection flag, ordered event ledger, every post-event capital snapshot, failures, and final state. Event IDs and ticks link each transition to its explanation.

Within a tick, arrivals precede lifecycle changes, financing, the action, the price move, interest, and constraint checks. Same-tick arrivals are explicitly usable. Reduction fees are checked against margin before granting fill-based relief. Successful recovery after a historical margin breach never changes the plan back to feasible.

Plan execution is deliberately **not atomic**. Reservation, dispatch, arrival, collateral pledge, disbursement, and action are separate events. Failure preserves completed steps and costs. The tool cancels a dependent action after an earlier failure; it does not invent a refund or collateral release. After a breach, subsequent states are a diagnostic continuation without simulated liquidation.

## Validation and scope

```sh
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

Tests cover double pledge, withdrawing pledged funds, delayed settlement, local margin failure despite positive aggregate equity, the plan/cost matrix, debt accounting, non-atomic failures, expired quotes, lifecycle separation, late recovery, invalid inputs, deterministic output, and CLI exit semantics.

Phase 0 establishes an executable explanation, not a sellable business. No trader interviews or outreach until Mo greenlights this demo. The commercial question remains whether a reachable buyer will pay for capital-state truth and planning without live credit; a negative answer is a reason to stop Lane 1 early.
