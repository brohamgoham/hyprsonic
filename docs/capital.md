# Reconcile real capital

`capital` fetches live observations and builds a scoped capital view. It never reads the Phase 0 fixture. The six states overlap; quantities retain their asset/network identity. Unknown amounts are `null` in JSON and `UNKNOWN` in text, never an invented zero.

## Run

Follow [account setup](observations.md) to enter your actual public addresses into ignored `.local/accounts.json` and set RPC environment variables. No venue API key or private key is used. The existing example has placeholders and deliberately refuses to run until edited.

```sh
cargo run --locked -- capital --config .local/accounts.json
cargo run --locked -- capital --config .local/accounts.json --json
```

Use `--explain` to print every holding and its classification ledger. The default display shows 20 holdings; JSON and the saved report always contain all collected holdings. Evidence and reports are stored privately under `.local/capital/<time>-<pid>/`.

```sh
# Inspect a saved decision without fetching anything or treating it as current:
cargo run --locked -- explain --report .local/capital/RUN/report.json
# Restrict the ledger to a claim ID copied from the report:
cargo run --locked -- explain --report .local/capital/RUN/report.json --claim CLAIM_ID
```

`cargo --offline` only disables dependency downloads. `capital` still contacts real APIs. There is no built-in funded reference account or fixture fallback.

| Exit | Meaning |
|---|---|
| `0` | Scoped reconciliation completed without detected evidence/rule/reservation errors; coverage limitations still apply |
| `3` | Missing/partial/stale data, unsupported mode, accounting conflict, local margin breach, or invalid reservation |
| `1` | Invalid input, configuration, application or journal failure |

**Every funding verdict remains `UNDETERMINED`.** An observation or a reported withdrawal ceiling is not a route, deadline guarantee, permission to spend, or destination receipt. Phase 3 will evaluate funding requests.

## Supported scope

| Account/data | Capital interpretation | Limits |
|---|---|---|
| EVM configured native/token balance | Settled quantity at RPC-reported finalized block, pinned and rechecked by hash | RPC trust remains. No fallback to `latest` when finality is unsupported. No gas/allowance/contract-control/open-order/private-pledge inference |
| HL explicit `disabled` mode, user/subaccount, default DEX, cross-only positions | Separate spot and perp claims; collateral excluding unrealized PnL, live tiered maintenance, initial margin, transfer floor and checked venue withdrawal ceiling | Versioned model has deterministic tests; own-account and matched UI validation pending. No HIP-3 enumeration, isolated positions, depositor withdrawals or executable risk prediction |
| HL `default`, unified, portfolio, DEX abstraction, vault, unknown mode | Observed holdings; unsupported constraints reported explicitly | Never assume `default` means standard. Shared spot/perp views cannot become two balances. No funding capacity is inferred |
| Polymarket positions + Gamma market lifecycle | Identified outcome-token holdings, lifecycle annotations, blocked token quantity | Offchain lifecycle and `redeemable` are not oracle finality, a payout amount, redemption receipt or credited cash. Authenticated orders and onchain CTF settlement remain unobserved |
| Matching Polygon holding wallet | Single wallet balance linked to the configured Polymarket identity | No second Polymarket cash balance is manufactured; private order commitments remain unknown |
| HL non-funding ledger | Last 24 hours, bounded inclusive pagination, historical event evidence and deduplication | Never added to snapshot balances. Newer-than-snapshot activity invalidates capacity. Does not establish destination credit or all liabilities |

These are connected-account facts, not a complete balance sheet. No data source proves account control or the absence of private liabilities. Pending proceeds remain unknown until supported settlement evidence can establish an amount. `pledged_reserved` labels its scope: a reported spot hold or computed position requirement is not all external obligations. Local reserves are a separate projection.

## Local reserves

Copy `config/reserves.example.json` to `.local/reserves.json`. Each reservation consumes a single claim in that claim's units. Copy the full stable claim ID from your actual report; no account alias or symbol-only matching is used.

```json
{
  "reservations": [
    { "id": "keep-buffer", "claim_id": "COPY_EXACT_CLAIM_ID_FROM_YOUR_REPORT", "amount": "25.000000" }
  ]
}
```

```sh
cargo run --locked -- capital --policy .local/reserves.json
```

These entries are labeled **user policy**, not venue locks. Duplicate IDs, missing claims, unknown capacity, overflow, and reservations exceeding the observed ceiling fail explicitly. Venue spot holds reduce that ceiling first. Checking a reservation against a finalized wallet balance is only an upper-bound check; it does not establish free capacity when external commitments are unknown. There is no persistent reservation service or concurrent plan locking yet.

## Rules, precision and evidence

Rule versions are embedded in each report. For supported HL cross accounts, maintenance integrates the live per-instrument tier rates; initial margin uses observed position leverage. The transfer floor is the greater of initial margin and 10% of notional. Each segment/position requirement rounds upward to six decimal places; the report declares this choice. The independently computed maintenance must agree with the venue within the conservative segment-rounding bound. Notional totals must match. Withdrawal above the model ceiling fails reconciliation rather than being silently rounded into agreement. These are current-snapshot checks, not stress simulations. [Margin rules](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/margining), [tier formula](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/margin-tiers).

The model requires an explicit standard account because unified and portfolio modes share balances across products. The mode is read again after enrichment to detect changes during collection. [Account modes](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes), [Info API](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint).

Polymarket lifecycle requests batch up to 25 condition IDs, with at most 500 conditions per run. Missing/duplicate/unrequested market records or a bound hit report incomplete coverage. A resolved service flag still does not prove a redemption has credited collateral. [Gamma API schema](https://docs.polymarket.com/api-spec/gamma-openapi.yaml), [resolution and redemption](https://docs.polymarket.com/concepts/resolution).

The default cross-source skew limit is 30 seconds; change it explicitly with `--max-skew-seconds`. `max_age_seconds` comes from account config. A finalized block can legitimately be older than those bounds: adjust the declared policy for the chain or accept an unavailable projection. No silent weakening of finality/freshness occurs. For endpoints without source timestamps, receive time only measures fetch age, not upstream data age.

Facts, inferred classifications, historical events, and user policy have different ledger kinds. Every known projection references the response evidence that supports it; computed results include their dependencies. Raw captures remain in `.local/`. The immutable run journal allows historical inspection; it is not yet a continuous event store or a receipt reconciler. No balances are mutated by historical activity, eliminating snapshot-plus-event deposit double counting.

## Remaining acceptance

Configure our own accounts, verify their modes and compare amounts against venue screens at matched times. If the owner's HL mode is unsupported, implement that mode before producing its capital capacity; do not change the wallet's account mode just to make a test pass. Complete settlement/receipt and authenticated commitment coverage only where the intended funding workflow requires it. See [Phase 2 evidence](phase2-report.md) and [roadmap](roadmap.md).
