# Observe real accounts

Phase 1 implements read-only observations from Hyperliquid, Polymarket predictions and EVM wallets. It does **not** reconcile the six capital views or decide whether a funding plan is feasible. Every report says `funding_verdict: UNDETERMINED` and `capital_reconciled: false`.

## Configure your accounts

Run from the repository root:

```sh
umask 077
mkdir -p .local
cp -n config/accounts.example.json .local/accounts.json
```

Edit `.local/accounts.json` with your public account addresses. Replace every `YOUR_*` placeholder or remove account entries you do not want to observe. The example is deliberately invalid until configured. Do not put private keys, seed phrases or API secrets into it.

- **Hyperliquid:** use the actual user/subaccount address, not an API agent wallet. The reader checks the account role and records its abstraction mode. A vault is observable, but no depositor withdrawal model is implied.
- **Polymarket:** use the holding/profile wallet that actually owns the positions. The address returned with each position must match. An empty response cannot establish identity or prove zero cash. This phase observes the documented positions v1 endpoint; it does not authenticate or discover every wallet linked to a login.
- **EVM:** configure the holding wallet or funding wallet, expected chain ID, RPC environment-variable name, and token contracts/decimals. Remove tokens you do not need. Chain ID, token code and decimals are checked before token amounts are accepted.

Set RPC URLs for the configured EVM readers. These examples use public endpoints; you can use your own HTTPS RPC provider instead:

```sh
export HYPRSONIC_POLYGON_RPC=https://polygon-bor-rpc.publicnode.com
export HYPRSONIC_ARBITRUM_RPC=https://arbitrum-one-rpc.publicnode.com
```

RPC URLs are read from environment variables so provider credentials do not need to appear in CLI arguments or configuration. URLs and provider error text are omitted from the journal. No venue credentials or signing keys are consumed by this implementation. Public endpoints can be unavailable or rate-limited.

The example's Polygon pUSD contract comes from [Polymarket's contract list](https://docs.polymarket.com/resources/contracts). The Arbitrum USDC address comes from [Circle's contract list](https://developers.circle.com/stablecoins/usdc-contract-addresses). Token identity remains chain + contract, not its display symbol. The runtime confirms code/decimals, not the token's legal backing, bridge eligibility or future behavior.

## Run

Build prerequisites: Rust 1.88+, Cargo, and a working C toolchain for the TLS backend; have CMake available for its native build. The current NixOS workspace has built this dependency successfully.

```sh
cargo run --locked -- observe

# Full machine-readable observation report:
cargo run --locked -- observe --config .local/accounts.json --json
```

Once dependencies are cached, add `--offline` before Cargo's `--` if needed. This prevents Cargo downloads; the **observe command still makes network requests**. `demo` and `replay` remain synthetic and offline.

`--json` writes only the JSON report to stdout. The journal path is printed to stderr. Account data is private even when the underlying addresses are public; redirect reports only into the ignored local directory if you save them.

| Observe exit | Meaning |
|---|---|
| `0` | Configured reads returned usable responses within the local age policy; this is not funding eligibility or full account coverage |
| `3` | One or more reads were unavailable, partial, stale, structurally unsupported, or inconsistent |
| `1` | Invalid configuration, journal failure or application failure |

Example failure from the actual CLI, with an unconfigured RPC variable:

```text
Funding verdict: UNDETERMINED (Phase 1; capital is not reconciled)
public_polygon_reference [evm] Unavailable | mode="not reported" | 0 facts / 0 responses
! MISSING_RPC_CONFIG: evm — Set the configured HYPRSONIC_* RPC URL environment variable.
```

It exits `3`; it does not substitute a zero wallet balance. The alias above identifies a public-reference smoke test, not an owner wallet.

## Coverage

This table describes the raw `observe` command. The newer [`capital` command](capital.md) adds margin metadata, bounded account history, market lifecycle enrichment, finalized wallet reads and scoped reconciliation.

| Reader | Implemented | Explicit limits |
|---|---|---|
| Hyperliquid | User role, abstraction mode, default-DEX perpetual account state, spot state, open-order payload/count | No HIP-3/subaccount enumeration, vault-deposit eligibility, margin-rule evaluation, marks stream or transfer history; default DEX and spot values must not be summed |
| Polymarket predictions | Paginated positions, exact consumed decimal values, position identity, reported valuation/P&L and redemption hints | No cash/commitment inference, authenticated orders, resolution confirmation, CTF redemption verification, v2 integration or portfolio reconciliation |
| EVM wallet | Chain verification, block-pinned native/ERC-20 balances, code/decimals checks, post-read block-hash recheck | No finality guarantee, allowances, gas-cost estimate, transfer tracking or source/destination reconciliation |

These readers may describe the same underlying holding through different APIs. There is no combined equity number. Polymarket `redeemable` is a hint, not a credited payout. HL fields are venue-reported observations; supported mode-specific interpretation is available through Phase 2's `capital` command.

The command observes only explicitly configured accounts. It cannot prove the absence of hidden off-chain obligations or account-specific permission restrictions. Known coverage gaps are reported separately from failures of the requested reads.

Polymarket pagination requests size threshold `0`, includes archived positions, and uses pages of 500 through the documented maximum offset of 10000. A full final page, duplicate position or malformed page produces partial coverage. Offset pagination cannot guarantee a single atomic snapshot under concurrent trading; this limitation is reported even after all pages return.

## Evidence and freshness

Each invocation creates `.local/observations/<time>-<pid>/` with numbered evidence files and `report.json`. On Unix, new journal directories use mode `0700`, files `0600`; symlink paths and collisions with existing runs are rejected. `.local/`, `.env*`, `.key` and `.pem` files are ignored by Git. Keep any other private configuration inside `.local/`.

Successful response JSON, safe request parameters, status, capture times, endpoint label and adapter version are persisted before parsing. Whitespace is normalized on serialization; decimal precision is retained. Failed responses record safe error metadata, not provider bodies. Facts reference their evidence file. Report references additionally carry source times/block hashes where established by the adapter.

`max_age_seconds` is a local age threshold checked after the reads finish. If source time is absent, freshness of the upstream materialized data is **unknown**; recent receipt time does not prove a recent underlying update. Excessive source age or future timestamps degrade the read status. Across venues this remains an observation window, not an atomic snapshot.

Requests use HTTPS with redirects disabled, a 5-second connect timeout, 12-second request timeout and 2 MiB successful-body cap. At most four accounts are read concurrently. HTTP 429/503 gets at most two retries with bounded backoff. A long or nonnumeric `Retry-After` is returned as a read failure rather than slept through or ignored. No other retry loop runs indefinitely.

Amounts use checked `i128` atoms plus decimal scale up to 38; values outside that range or unsupported numeric notation produce an explicit parse failure. JSON numbers never pass through binary floating point. This is observation precision support, not a claim that all possible uint256 values fit the model.

## Verify and next gate

```sh
cargo test --workspace --locked
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo fmt --all -- --check
cargo tree -p capital-core --locked
```

Tests are offline and deterministic; HTTP transport tests bind local loopback sockets. The core depends only on Serde and standard-library types, with `AccountSource`, `EvidenceStore` and `Clock` ports. It has no HTTP client, filesystem access, clock reads or venue SDK dependency. The application use case accepts account and clock ports; the composition root wires concrete readers and the private journal.

Live smoke checks are opt-in: run `observe` against addresses you explicitly choose. Public-reference connectivity has been exercised. **Own-account acceptance is still pending the owner's configuration**, so Phase 1 is implemented but its full own-account gate is not passed. After that check, Phase 2 adds capital reconciliation and supported rules. See [the Phase 1 report](phase1-report.md).
