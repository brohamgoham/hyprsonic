# Phase 2 — scoped live capital reconciliation

2026-09-18. **Implementation delivered; own-account and supported-rule acceptance still pending.** The `capital` command uses real API/RPC reads. It has no fixture input or fallback. The owner reported a successful run; the recorded acceptance evidence below uses explicitly configured public reference accounts, not owner-controlled wallets.

## Delivered

- Pure `capital-core` claim/state projections and a `ConstraintModel` contract. The core's only direct dependency remains Serde; no I/O, wall-clock reads, transport JSON or venue SDKs.
- Six overlapping views, explicit unknowns, chain/asset/holder identities, source evidence, local reservation policy, classifications and historical activity ledger. No summed portfolio cash number.
- Standard-cross HL margin model using live tier metadata, observed leverage and notional. Initial, maintenance and transfer requirements remain distinct; unsupported account modes fail explicitly. The mode is checked again after collection.
- Polymarket positions joined to bounded Gamma lifecycle reads. Outcome tokens, resolution status, redemption hints and actual wallet balances remain different facts. No pending payout is manufactured from marked value.
- Finalized-block wallet reads with chain/code/decimals validation and hash recheck. A missing finalized block does not fall back to latest. Same-holder Polygon/Polymarket views are linked without making duplicate cash.
- Bounded HL non-funding ledger history, inclusive pagination and event deduplication. Events never increment snapshot balances. Activity newer than the snapshot prevents capacity inference.
- `capital --explain`, private JSON reports and offline historical `explain --report ... [--claim ...]` inspection. The CLI shows collection time separately from pure reconciliation time.

Run instructions, limits, policies and primary rule references are in [the capital guide](capital.md). [Phase 2T](tui-proposal.md) proposes a Ratatui inspection workspace; no TUI is implemented in this revision.

## Actual live run

Final implementation run: `capital --config .local/public-reference-accounts.json --json`, with the configured public Polygon RPC. Configuration, addresses, balances, raw responses and complete report remain ignored under `.local/`.

| Reference account scope | Responses | Observed evidence |
|---|---:|---|
| HL public vault reference | 8 | Role/mode, perps, spot, orders, 234 instrument margin schedules, 102 recent non-funding ledger events and final mode recheck |
| Public Polymarket holding wallet | 7 | 168 outcome positions and lifecycle enrichment for their conditions |
| Same holding wallet on Polygon | 7 | Finalized native/pUSD balances, chain/token validation and canonical block recheck |

Result: **22 responses, 171 claims, 1,131 ledger entries**. Two wallet claims had supported settled projections; outcome positions contributed no credited cash. Network collection plus persistence took **1,849 ms**; pure reconciliation took **8,714 µs**. This single run is not a latency percentile benchmark or performance guarantee.

The run intentionally returned exit **3** because the public HL reference was a vault in `default` mode. Its mode/depositor constraints have no supported funding model. The real failure moment was:

```text
Funding verdict: UNDETERMINED
! [HL_MODE_UNSUPPORTED] public_hlp_reference: Only explicit disabled/standard
user or subaccount mode has a local margin model. default, unified, portfolio,
DEX abstraction and vaults remain observed-only.
```

This is a real coverage refusal, not a synthetic loss or a failed money movement. The supported standard-cross margin model has **not** yet been reconciled against an owner account/venue screen. No evidence is relabeled to claim that gate passed.

Saved-report inspection ran successfully with a selected real claim and exit `0`; output began `HISTORICAL REPORT — not current funding availability`. Journal/report permissions were checked as `0700` directories / `0600` files. Final private run timestamp: `1789711986307`; no account capture is included in this commit.

## Verification

- `cargo test --workspace --offline --locked`: **62 tests passed**: 3 core primitives, 15 capital/risk invariants, 7 adapter/transport unit tests, 5 new capital adapter/CLI checks, 16 observation integration tests, 16 existing Phase 0 regressions.
- `cargo clippy --workspace --offline --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo tree -p capital-core --depth 1`: Serde only.
- Public-reference live command and saved-report explanation exercised; all requested services responded.

New tests cover local margin failure despite capital elsewhere, tier boundaries and upward rounding, decimal overflow/precision, shared collateral modes, isolated/unknown-mode refusal, asset/network distinction, stale/skewed/partial observations, duplicate claims, double pledge/spot holds, duplicate reservation IDs, settlement hints excluded from cash, duplicate/conflicting/newer events, finalized-block refusal, live metadata structure, lifecycle identity gaps and invalid CLI setup. Controlled test inputs are accounting edge cases, not new runtime fixtures or a new synthetic product demo. Transport tests use local loopback servers.

## Remaining gates

1. Replace placeholders in the owner's local account config and verify actual identities/modes. The current checked config still contains placeholders. Public-reference connectivity does not finish Phase 1 acceptance.
2. Match the supported account's balances, margin and withdrawal constraints to venue values at a matched time. If the actual mode is unsupported, extend the model with evidence before evaluating its capacity. Do not infer standard mode from `default`.
3. Resolve authenticated commitments and settlement/receipt coverage needed for the chosen funding path. Today these stay explicitly unknown: private obligations, Polymarket open orders, onchain resolution/redemption receipts, in-flight transfer amounts and destination credit. There is no complete liability ledger or continuous reconciliation service.
4. Then evaluate Phase 3's real funding request. A convenient TUI may help inspect the data, but neither successful fetches nor a UI make a funding plan feasible.

No API key is required by this implementation. A private RPC subscription can replace the public RPC through the existing environment variable. Authenticated venue coverage is a separate adapter decision: verify actual credential powers before asking for a key. Private keys, seed phrases and trading credentials are not inputs to this phase.
