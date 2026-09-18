# Proposed Phase 2T — capital inspection workspace

Status: proposed on 2026-09-18 after the live capital CLI was exercised. No Ratatui dependency, TUI command or keyboard interface is implemented yet. This is a usability slice of the capital workflow; it does not complete Phase 3 funding plans or Phase 4 plan monitoring.

## Job

Open HyprSonic, inspect the connected accounts, identify a capital restriction, and follow it to evidence without reading a long JSON document. Use this workspace while verifying our own accounts before selling anything.

| View | What it answers |
|---|---|
| Accounts | Which accounts were reached, in what mode, how recently, and with which coverage gaps? |
| Capital | What is settled, reported withdrawable, unrealized, pending, reserved or blocked, in the actual asset units? |
| Explain | Why is this amount known or unknown? Which constraint, policy and response support it? |

The first version should support keyboard selection, filtering, explicit refresh, refresh progress, scrollable evidence, and a persistent freshness/coverage indicator. Full claim identity should be available on demand. Preserve the CLI and JSON output for scripts. Detailed key bindings and the enhanced usage guide belong with the implemented UI.

## Architecture recommendation

Use Ratatui as a presentation adapter over the same application use cases and pure capital core. Keep one authoritative application state and explicit messages for input, refresh completion, failure and freshness expiry. Network collection runs on workers; rendering never waits on HTTP, RPC, journal writes or margin computation. Ratatui documents several compatible application patterns and leaves event handling to the application/backend. [Application patterns](https://ratatui.rs/concepts/application-patterns/), [event handling](https://ratatui.rs/concepts/event-handling/).

Start with manual refresh and at most one in-flight collection. If periodic refresh is added, respect provider rate limits and show the actual cadence. Reject results from superseded refresh generations. Retain the last snapshot visibly as historical after errors; never keep presenting its amounts as fresh. A slow venue must not freeze navigation or quit handling. A dropped or incomplete capital event requires resynchronization before dependent claims can regain valid coverage.

Do not move venue parsing, reserve arithmetic or margin rules into widgets. A UI action cannot bypass the same unsupported-mode, settlement, double-pledge or freshness checks enforced by the CLI. Colors supplement text labels; an unknown value must remain visibly different from zero and from known blocked capital.

## Acceptance proposal

- The owner can find a holding, its blocker, and its exact evidence without switching to raw JSON.
- CLI and TUI show the same classifications for the same captured observation and policy.
- Navigation, resize and quit stay responsive through a slow request, failed endpoint and partial refresh.
- Refresh failure and age expiry visibly invalidate current availability; subsequent valid refresh recovers it.
- Terminal state is restored on normal exit, error and panic; a narrow terminal remains usable.
- Account captures and credentials remain private. No signing, execution, market-making screen or invented funding verdict is introduced.

This can help perform the pending own-account verification. It cannot substitute for that verification or turn observed balances into a finished fund-by-deadline product. The subsequent product step remains Phase 3: one supported funding request with real route dependencies.
