# HyprSonic: fund the next move

Product decision, 2026-09-14. The [full delivery plan](roadmap.md) defines implementation phases and acceptance gates. Only the [Phase 0 synthetic demo](phase0-report.md) is implemented today; live connections, monitoring, and the workflow below are not shipped capabilities. The agreed release sequence is to test with our own wallets and tune through repeated use before considering an external paid pilot.

## The product

**Connect your trading accounts and wallet. Tell HyprSonic where you need capital, by when, and which buffers to preserve. Get a funding plan, follow its progress, and learn immediately when its assumptions stop holding.**

The first customer hypothesis is a human trader or small trading team that repeatedly reallocates its own capital across prediction markets and perpetuals. The recurring job is deciding what can move without impairing another account, then checking that the money actually arrives. A trader who rarely reallocates has little reason to subscribe.

The initial product is a capital operations workspace. Its main screen is a funding request and its live status. The six capital views explain the answer; they are not the entire product. The smallest interface has three views: connected accounts and coverage, a funding request with alternatives, and a timeline of confirmations or blockers. The CLI will establish that workflow before a minimal web UI.

## Which thesis wins?

Keep the team's capital-state and fail-loudly thesis. It is the sharper first product boundary. Keep the earlier cross-venue thesis as the direction of travel: independent planning across accounts, with provider financing only if usable access eventually exists. Neither thesis establishes willingness to pay.

Two corrections matter:

1. Truth must change a decision. Seeing that a payout is pending is weak value if the venue already shows it. Showing that a proposed withdrawal breaks a protected buffer, offering a usable alternative, and tracking the resulting transfer is a complete job.
2. A one-off plan is unlikely to retain users. Saved capital policies, changing obligations, plan invalidation, and reconciliation must earn repeat use. If planning needs live credit to become valuable for this customer, stop this version of Lane 1 rather than silently starting a lending business.

The weakest commercial assumption is the number of reachable people who actively use both Polymarket and Hyperliquid and face this problem repeatedly. API access and a correct engine cannot validate that assumption. No claim is made that this market is empty or that the product will be profitable.

## Connect X and Z, do Y

First connections: **a Hyperliquid account, a Polymarket prediction-market account, and a wallet on supported chains**. These are explicit account identities, not an assumption that one address represents the same account everywhere.

The first funding request is: "Put a specified amount of the destination's collateral into this Hyperliquid account before my deadline; preserve my source cash floor and destination margin buffer under my selected price scenario."

The workflow:

1. Connect addresses for observation. Identify the actual Hyperliquid account and the Polymarket holding wallet. Show permissions and missing coverage before using balances in a plan.
2. Reconcile observed holdings, venue constraints, open commitments where accessible, liabilities, and transfers. Show settled, withdrawable, unrealized, pending proceeds, pledged/reserved, and delayed/blocked separately, with source and observation age. Unknown is never a zero balance or an unencumbered balance.
3. Enter a destination amount, deadline, protected source balances, and an explicit stress scenario. Start with a collateral top-up to support existing positions. Sizing an arbitrary new trade requires additional instrument and initial-margin rules and is a later increment within the same product.
4. Compare routes from already available wallet funds and eligible venue withdrawals. Add sell/reduce alternatives when quote and rule coverage can support them. A claim awaiting redemption is a separate dependency. Financing is unavailable unless an actual provider supplies eligible, actionable terms; the old synthetic quote stays in the demo.
5. Show expected cost, amount delivered, timing assumptions, surviving buffers, and each prerequisite. Rank feasible candidates within the supported route set. Never call that a globally optimal route.
6. The trader performs approved steps in the native venue or wallet. HyprSonic observes receipts, destination credit, and policy changes, and re-evaluates the request. A click or manually checked box does not establish settlement.

The resulting answer might be: "This payout cannot count toward your deadline. Your wallet route is conditional on destination credit. With the selected price move, the destination buffer would still be insufficient; reduce the requested amount or reduce existing exposure." Dollar examples in the old demo remain synthetic. The live workflow must use actual observations and label user-entered scenarios separately.

The first real-data increment may correctly return **UNDETERMINED**. It is an engineering milestone until it can close this entire workflow for a supported account configuration; fetching prices alone is not a product.

## What are the 'things' we connect?

| Connection | What it contributes | Limit we must preserve |
|---|---|---|
| Hyperliquid account and account mode | Positions, local collateral constraints, withdrawable amounts where meaningful, marks, account events | A frontend or DEX name is not automatically an independent collateral pool |
| Polymarket prediction-market holding wallet | Positions, resolution/redemption evidence, collateral balances and accessible commitments | Marked position value and redeemable claims are not destination collateral |
| Chain wallet | Token balances, gas availability, receipts and confirmation evidence | Chain, token contract, decimals and controller are part of identity |
| Supported transfer route | Input/output assets, fees, estimates, prerequisites and receipt references | Provider quote, source confirmation and destination credit are distinct |
| Financing provider, later | An account-specific quote, collateral control, debt and repayment obligations | No quote or eligibility means no financing contribution |

Current Hyperliquid documentation describes standard, unified and portfolio-margin modes with different collateral treatment. Unified and portfolio modes require attention to spot-state balances; per-DEX states cannot simply be summed. That makes account mode a required model input. [Account modes](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes).

trade.xyz deploys markets on Hyperliquid. It can expand instrument coverage through the Hyperliquid adapter; adding its logo does not by itself establish another independent venue or pool. Fullstack's retrieved public page says trading is coming soon and establishes no usable integration contract for this product. [trade.xyz](https://docs.trade.xyz/), [Fullstack](https://www.fullstack.trade/).

Polymarket currently documents pUSD collateral on Polygon. Its position API exposes redemption-related fields, and its bridge API offers estimated output, costs and checkout time. None alone establishes spendable collateral at Hyperliquid. [Collateral](https://docs.polymarket.com/concepts/pusd), [positions](https://docs.polymarket.com/api-reference/core/get-current-positions-for-a-user), [quotes](https://docs.polymarket.com/trading/bridge/quote).

Kalshi remains outside live implementation in this lane. The existing synthetic lifecycle stays available, and commercial access remains unmet. Polymarket perps, extra venues, and additional account modes do not enter the first slice simply because APIs exist.

## Why pay, and why might nobody pay?

The proposed buyer pays to make repeated reallocations with less manual checking, less avoidable prefunding, and fewer funding mistakes. Those are hypotheses to measure. We cannot truthfully book an "avoided liquidation" as savings from an alert, or call all idle cash waste.

Loris already advertises consolidated positions, funding P&L, margin and equity history. Basic aggregation and margin alerts are insufficient differentiation. Our proposed difference is a deadline-specific funding request, source constraints, settlement dependencies, and follow-through. The retrieved page does not prove competitors lack that workflow. [Loris portfolio](https://loris.tools/portfolio).

FalconX describes financing and portfolio netting for supported accounts within a prime relationship. Amplifi advertises Polymarket leverage and future cross-venue expansion. They reinforce the distinction between observing capital and being able to lend against it; they also could absorb this workflow. Neither establishes a HyprSonic integration or customer opportunity. [FalconX](https://www.falconx.io/newsroom/falconx-introduces-prime-brokerage-margin-financing-for-trading-on-hyperliquid), [Amplifi](https://amplifi.finance/).

The initial selling hypothesis is a subscription per workspace for persistent monitoring, saved policies, history, and reconciliation. The public Rust engine can remain inspectable; managed operation must provide the recurring service worth paying for. No licensing change is made in this phase.

**Proposed price experiment: $250 per workspace per month**, including a small team. This is a test price, not researched willingness to pay. Twenty retained paying workspaces would be $5,000 monthly revenue before costs; there is no evidence yet that we can acquire or retain them. Hosting, feed/RPC costs, support, and adapter maintenance must fit that revenue. Builder fees and hypothetical lending commissions are not the model.

## Commercial gates

First complete the own-wallet trial and tuning gates in [Phases 5–6](roadmap.md#phase-5--test-with-our-own-wallets). No outreach occurs until Mo explicitly approves it after those gates. The following is a proposed future experiment, not permission to recruit:

- Start with five users/teams that already make this type of cross-venue funding decision weekly. A test account created solely for our demo does not count.
- Over four weeks, seek at least three that use the tool on real decisions in three separate weeks, identify a concrete decision improved or manual task removed, and then pay the test price to continue without requiring new credit.
- Record onboarding time, unsupported coverage, incorrect or stale classifications, decision completion rate, and support cost. A critical false feasible result blocks rollout until understood and corrected.
- Repeated decisions but no payment means a selling or value problem to investigate. Infrequent use, no meaningful alternatives, or a requirement that we supply credit are reasons to stop or explicitly change Lane 1. Do not respond by adding venue logos.

These thresholds are chosen project gates, not statistically established market validation. Cantina remains separate; this document makes no claim about its revenue.

## Build order and latency

Phase 0 is frozen as regression evidence. Next: real observations and coverage through the [ports and adapters design](architecture.md), then one end-to-end funding request with continued observation. The [delivery plan](roadmap.md) expands this into connection, reconciliation, planning, monitoring, own-wallet trial, tuning and commercial-decision gates. A generic high-performance engine built independently of those two adapters would postpone the hardest semantic questions.

Fast invalidation matters: a stale plan can be misleading even if it originally calculated correctly. Measure upstream age, ingestion delay, recomputation, and time to display separately. Transfer and settlement duration are external dependencies. Local microsecond computation cannot remove them.

The initial performance target is p95 under 100 ms from a normalized observation arriving to a revised decision for the supported small portfolio. It is a proposed local benchmark target, not an achieved result or end-to-end guarantee. Upstream freshness and completeness must remain visible even when that target is met. Rust, bounded queues and incremental updates are appropriate; specialized nodes or a Reth fork require evidence from profiling and an actual missing capability.

Next-phase completion is a usable funding workflow on real accounts, with reproducible evidence and honest unknowns. More synthetic examples do not satisfy that gate.
