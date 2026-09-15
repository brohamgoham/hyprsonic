# Phase 1 — real observations and the core boundary

**Implementation delivered; own-account acceptance pending.** This increment adds the `observe` command, a two-crate boundary and real read-only adapters. The owner has not supplied an own-account configuration yet. Public reference reads validate connectivity and response handling, not the owner's wallets or a completed funding workflow.

## Run

Follow [the setup guide](observations.md), then:

```sh
cargo run --locked -- observe --config .local/accounts.json
```

No keys, signing, execution, funding quotes or capital movements are involved. All live reports remain `UNDETERMINED` for funding. The original synthetic `demo` and `replay` commands continue to work.

## What changed

- `capital-core`: checked decimal amounts and account identities, collateral-domain placeholder, observation/evidence/coverage types, and `AccountSource`, `EvidenceStore`, `Clock` ports. Its only direct dependency is Serde.
- Application: observation use case with injected account/clock ports and up to four account workers, stable result ordering, end-of-run age checks, text/JSON output and explicit exit codes.
- Hyperliquid: role/mode discovery plus default-DEX and spot observations. An agent or missing account does not become an empty funded account.
- Polymarket: position pagination, identity/duplicate checks, precise values and redemption hints. A missing page does not produce a complete count or zero cash.
- EVM: expected-chain check, pinned-block balances, token code/decimals validation and a final block-hash check. Provider URLs/errors are redacted from evidence.
- Local evidence: ignored configuration and journals; exclusive private run directories/files, source references and safe error metadata.

The six reconciled capital views, real margin constraints, transfer routes, monitoring and UI are later roadmap phases. There is no total portfolio cash/equity calculation in `observe`.

## Live evidence

A public-reference run reached all configured services and exited `0`:

| Reference reader | Valid responses | Observations | Result |
|---|---:|---:|---|
| Public Hyperliquid vault reference | 5 | 8 facts; mode reported as `default` | Requested reads complete; no depositor/risk model |
| Public Polymarket reference | 1 | 168 positions; 673 facts | Requested page complete; lifecycle/cash coverage limited |
| Same public holding wallet on Polygon | 7 | Native and pUSD token balances | Chain/code/decimals and pinned-block recheck passed |

The final recorded invocation took **3,773 ms** for 13 responses. This is one observation of network calls plus local persistence, not a performance guarantee or the future core-planning benchmark. Raw financial values and identities remain in ignored local evidence; sanitized structural captures are under `tests/captures/`.

A separate real CLI invocation with a missing RPC variable exited `3`:

```text
Funding verdict: UNDETERMINED (Phase 1; capital is not reconciled)
public_polygon_reference [evm] Unavailable | mode="not reported" | 0 facts / 0 responses
! MISSING_RPC_CONFIG: evm — Set the configured HYPRSONIC_* RPC URL environment variable.
```

This demonstrates a failed observation without inventing a wallet balance or performing a failed financial transaction. Network-restricted execution also produced unavailable reads, and a permitted network run succeeded.

## Validation and remaining acceptance

The test suite covers exact decimals, overflow, invalid account config, account roles/modes, empty versus failed reads, position pagination bounds/duplicates, partial-page failures, RPC error redaction, private journal behavior, chain/decimals mismatch, pinned blocks/reorgs, application ports/freshness, HTTP retry/redirect/body limits, sanitized API captures and the existing 16 Phase 0 cases.

See the commit containing this report for the implementation revision. Validation on the final implementation:

- `cargo test --workspace --offline --locked`: **41 tests passed** (3 core, 6 adapter/transport unit, 16 observation integration, 16 Phase 0).
- `cargo clippy --workspace --offline --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Core dependency tree: Serde only; no networking/runtime/filesystem dependency.
- Final live reference invocation: exit `0`; missing-RPC CLI case: exit `3`.
- Local evidence files/directories and Git exclusions checked. HTTP unit tests used local loopback sockets; no external network is required by the test suite.

 No owner wallet data or unrelated research is part of the commit.

Remaining gate: configure and run the owner's actual HL account, Polymarket holding wallet and required chain reads; inspect identity, source coverage and failures. No real-account reconciliation or funding trial is claimed. The [roadmap](roadmap.md) retains this pending acceptance instead of declaring Phase 1 fully complete.
