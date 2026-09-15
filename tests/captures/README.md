# Sanitized adapter captures

These are structural test inputs derived from successful public-reference endpoint reads during Phase 1. They are not demo portfolios, owner accounts or unmodified live financial data.

- `hyperliquid-account.sanitized.json`: `clearinghouseState` response. Numeric strings are replaced with `0.0` and source time with `1000`. The captured account had no positions in the default DEX response. Keys and container shapes are retained.
- `polymarket-positions.sanitized.json`: two rows from a positions response. Only the adapter's consumed fields are retained. Wallet, condition and token IDs are replaced with test identities. Financial values are replaced with fixed test numbers. Redemption booleans retain their source type/value.

Raw reference captures remain in the ignored local evidence journal. These sanitized files check response decoding and precision semantics; live connectivity and real coverage are recorded separately in the Phase 1 report. Constructed fault cases in the tests are explicitly test data.
