# HyprSonic delivery plan

**Build it. Use it with our own wallets. Tune it before selling it.**

This is the execution plan agreed on 2026-09-14. It expands the earlier architecture milestones and governs sequencing. The [product](product.md) and [architecture](architecture.md) documents define the scope and technical boundaries. Phase 0 is complete. Phase 1 readers/core and a scoped Phase 2 reconciliation are implemented with public-reference live evidence. Own-account acceptance remains pending; development of Phase 2 does not waive the Phase 1 gate. Phases 3–7 remain planned.

## Finish line

Using the owner's actual Hyperliquid account, Polymarket prediction-market holding wallet and supported chain wallet, HyprSonic can answer:

> Can I fund this destination by this deadline, preserve my source reserves, and maintain the destination buffer under my selected scenarios?

We can inspect the evidence, perform any chosen money movement ourselves in the native wallet or venue, observe its progress in HyprSonic, and see the result change when dependencies fail. Repeated internal use must establish that this is useful and reliable before any external pilot or selling.

This plan authorizes software development and read-only observation of accounts explicitly configured for the project. It is not an instruction to open positions, sign messages or move money. The owner chooses and performs the financial actions during the internal trial. No execution/signing capability is part of this release.

## Phase map

| Phase | Result you can use | Depends on | Status |
|---|---|---|---|
| 0 — Capital model demonstration | Replay the aggregate-equity/local-margin failure | — | **Shipped** |
| 1 — Connect real accounts | Inspect actual responses, account identities and data coverage | 0 | Implemented; own-account acceptance pending |
| 2 — Reconcile capital | Inspect capital states and account constraints with evidence | 1 | Scoped implementation; own-account/rule acceptance pending |
| 2T — Capital inspection TUI | Navigate holdings, blockers and evidence using Ratatui | 2 implementation | Proposed; not implemented |
| 3 — Plan one funding request | Compare supported ways to fund an existing HL account | 2 | Planned |
| 4 — Follow the plan | Monitor decisions and confirmations in a minimal workspace | 3 | Planned |
| 5 — Own-wallet trial | Complete and reconcile the workflow with our accounts | 4 | Planned |
| 6 — Tune and release internally | Repeat the job reliably without hand-editing state | 5 | Planned |
| 7 — Commercial decision | Explicit go/stop on a small external paid pilot | 6 + owner approval | Gated |

Phases are acceptance gates, not promises of calendar dates. Set an estimate for each implementation phase after its input/API spike; revise it when actual account coverage changes the work. A blocked route must be reported, not replaced by fixture numbers to keep a schedule.

## Phase 0 — preserve the proof

Already shipped: synthetic three-venue portfolio, four funding alternatives, delayed settlement plus price stress, non-atomic step failures and an event ledger. Sixteen tests pass; the recorded demo runs in under five minutes. See the [completion report](phases/done/phase0-report.md).

Keep this regression evidence. Do not extend its arbitrary ticks, fees, rules and loan into a production model. Later tests can use captured responses and controlled faults; those are explicitly test inputs, not an expanded product demo.

## Phase 1 — connect real accounts and establish the core boundary

Current evidence: [Phase 1 implementation report](phases/done/phase1-report.md). Configure the owner's addresses using [the observation guide](observations.md) to finish the own-account gate. Public-reference connectivity does not waive that requirement.

**Deliverable:** a CLI observation command reads configured accounts and shows exactly what it reached. This is the first implementation phase, not a standalone generic-engine project.

Work:

- Introduce `capital-core` and the `hyprsonic` application/adapter crate. Keep venue clients, I/O, credentials and runtime dependencies out of the core.
- Define checked asset quantities, account and collateral-domain identities, observations, coverage and errors. Wire `AccountSource`, `EvidenceStore` and `Clock` through application use cases; add the other ports when consumed.
- Add Hyperliquid and Polymarket REST adapters plus the necessary chain wallet reads. Resolve actual holding accounts and discover the HL account mode. Validate the selected token/chain identities.
- Use public reads where sufficient. Document any missing authenticated coverage and actual credential capabilities; credentials do not enter the core, CLI arguments or evidence journal. Do not silently upgrade access to make a read work.
- Add local configuration, explicit network timeouts, rate-limit backoff, bounded responses and complete pagination. Ignore local config, journals and secret files in Git before the first wallet is configured. Provide a tracked example containing placeholders only.
- Save raw response evidence locally with receive time, source time/block if provided, endpoint, adapter version and coverage. An account being empty, inaccessible or unsupported must be distinguishable.

Exit gate:

- Both venue endpoints and required wallet reads are exercised with explicitly configured identities; no hardcoded funded demo account.
- An operator can inspect account identity, mode, assets and source freshness without changing code.
- A missing permission, timeout, exhausted pagination bound or unknown account mode produces a useful coverage error. A failed read never becomes an empty portfolio.
- Core-only build/test succeeds without network SDKs. Adapter contract tests use sanitized captures; an opt-in network smoke report records what was actually reached and when.

Evidence: phase report, dependency-boundary check, sanitized adapter tests and a private observation journal. If user account configuration is unavailable, a public reference account can validate connectivity only; it does not pass the own-account acceptance gate.

## Phase 2 — reconcile capital and constraints

Current implementation: [`capital` and `explain`](capital.md), with [validation and remaining gates](phases/phase2-report.md). The standard-cross HL model is deliberately limited; the owner's actual mode has not been selected or validated. Authenticated commitments, onchain prediction redemption and destination receipts remain coverage gaps, not assumed zero balances or completed events.

**Deliverable:** a live, scoped capital view backed by observations rather than hand-entered balances.

Work:

- Build settled, withdrawable, unrealized, pending proceeds, pledged/reserved and delayed/blocked projections over unique holdings. Show unknown states explicitly; never sum overlapping views.
- Reconcile positions, collateral, accessible order commitments, transfers and known liabilities. Preserve externally unobservable obligations as a declared coverage limit; user declarations remain labeled assumptions.
- Implement the first observed HL account mode with versioned margin rules and independent check cases. Observe other modes without producing supported funding verdicts until their models are implemented.
- Separate prediction positions, resolution evidence, redemption eligibility and credited collateral. Preserve token/chain differences through every conversion.
- Join snapshots and events without counting the same capital twice. Detect duplicate observations, stale data, gaps, incompatible timestamps and conflicting sources.
- Make user reserve policies distinct from venue margin rules. Persist explicit precision/tolerance choices and record rounding adjustments.

Exit gate:

- On our supported account configuration, each displayed amount traces to evidence or a labeled assumption. Compare venue-displayed values at a matched observation time; explain every discrepancy within declared precision.
- Tests cover shared collateral pools, asset precision, unknown commitments, duplicate events, partial pagination and snapshot/event reconciliation.
- Pending, redeemable, pledged and in-flight amounts cannot silently enter free funding capacity. Unsupported model coverage blocks the affected verdict.
- The supported-account matrix states what is live, what is observed only, and what is unsupported.

Evidence: reconciliation report, rule version references, coverage matrix and an explain command/output. A balance screen alone completes this phase, not the product.

## Phase 3 — answer one funding request

The proposed [Phase 2T workspace](proposals/tui-proposal.md) can make account verification easier before or alongside this work. It does not replace funding-plan or monitoring acceptance gates.

**Deliverable:** request collateral at an existing Hyperliquid account by a deadline while preserving source reserves and a selected stress buffer.

Work:

- Implement `FundAccount` with explicit destination asset/amount, deadline, source reserve policy, horizon and stress assumptions. Arbitrary new-trade sizing stays outside this first request.
- Verify actual source/destination routes. Start with available wallet funds and supported withdrawals; model every intermediate asset conversion, receipt and destination-credit dependency.
- Add `RouteSource`, required market inputs and pure constraint evaluation. Report route quotes as estimates, with capture time, expiry if supplied, cost components and timing uncertainty.
- Represent each plan as steps with dependencies and resources. Local reservations prevent conflicts among our plans but do not claim to lock money externally.
- Evaluate constraints at intermediate steps and across the selected scenario, not only at the final balance. Unknown completion time remains conditional/undetermined.
- Rank candidates within the supported route set by user policy. Show amount delivered, expected cost, source balance remaining, destination buffer and first failure/blocker.
- Add sell/reduce candidates only after quote, fees, position and rule coverage supports them. Otherwise show them as unavailable with the reason. Live financing remains unavailable.

Exit gate:

- At least one supported candidate and one justified rejected/unknown candidate are demonstrated from actual observations, with any hypothetical stresses separately labeled.
- `FEASIBLE_UNDER_ASSUMPTIONS`, `CONDITIONAL`, `INFEASIBLE` and `UNDETERMINED` have distinct UI/JSON meanings. A transfer-dependent plan cannot appear already funded.
- Quote expiry, a missed deadline, insufficient gas, unavailable source capital and local margin failure produce actionable explanations.
- Every decision is reproducible from saved evidence, policies, rules and scenario inputs. Same inputs produce the same result.

Evidence: private real-account plan report and sanitized decision tests. A route unavailable to our accounts is a blocker to resolve by narrowing supported scope, not a reason to advertise a fictional route.

## Phase 4 — watch the plan in a minimal workspace

**Deliverable:** a locally runnable workspace with three views: accounts/coverage, funding request/alternatives, and plan timeline. The owner can operate it without writing JSON.

Work:

- Preserve the CLI for automation and diagnosis. Add a thin local web UI over the same use cases; bind locally by default and keep sensitive state out of third-party analytics.
- Add continuous observation using streams where supported and polling where necessary, with explicit cadence. Re-evaluate only affected plans while preserving complete capital events.
- Add `ReceiptSource`, destination-credit checks, in-app plan-change alerts, saved requests and an append-only evidence store.
- Link a user-supplied transaction/transfer reference to a plan. A manual "done" action is a note, not proof of completion. The user still operates the native wallet or venue.
- Recover from process restart, laptop sleep, network disconnect, sequence gaps and stale quotes. Resume after reconciliation, not by assuming nothing changed while offline.
- Persist plan revisions and breach history. Late recovery does not rewrite a failed deadline into success.

Exit gate:

- A request goes from creation through observed prerequisites to destination confirmation using the UI.
- New data that breaks a plan visibly invalidates it; stale or missing data removes the previous feasible indication.
- Restart/sleep/disconnect tests preserve history and produce a resynchronized state. Secrets are absent from logs, exports and tracked files.
- Benchmark the declared small-portfolio workload: 10 accounts, 100 positions, 20 candidate plans. Target p95 under 100 ms for normalized observation to revised decision; record hardware, event rate and p50/p95/p99. Separately measure upstream age and UI delay. These are targets, not shipped claims.

Evidence: local walkthrough, recovery test results and benchmark report. This phase makes the workflow usable enough for internal trials; it does not establish production reliability.

## Phase 5 — test with our own wallets

**Deliverable:** an internal trial report based on our actual accounts, using the [own-wallet runbook](own-wallet-testing.md).

Start with observation and reconciliation of existing positions. The owner sets supported accounts, an amount budget, fee cap, preserved balances and a stop condition before any manual money movement. Existing suitable activity can supply evidence; no new leveraged position or intentional financial loss is required to test the software.

Required cases:

1. Reconcile initial source holdings, commitments and destination collateral against the venue/wallet.
2. Complete one actual supported funding route and verify destination credit, actual fees and final account constraints.
3. Ask for an amount or deadline known to be infeasible; confirm HyprSonic explains the blocker without submitting anything.
4. Observe a real pending/redemption/transfer state when available. If the real event has not occurred, keep that live case pending; a recorded replay may test behavior but is not credited as real evidence.
5. Interrupt our local observer, resume it, and confirm the plan becomes stale then reconciles. Do not disrupt a venue or deliberately strand funds.
6. Change a policy or use ordinary external account activity to invalidate a plan. Confirm the old verdict is not retained.

Exit gate:

- At least one real route is completed and fully reconciled. No unexplained capital discrepancy or false feasible decision remains open.
- Any unavailable live lifecycle case is explicitly excluded from the release's supported coverage until verified. The core funded-account workflow cannot be waived.
- The owner can explain what the product changed about a decision and what still required manual work.
- Private evidence stays local; only redacted summaries and carefully sanitized captures are candidates for publishing.

Own-wallet success proves our tested scope, not general user demand or universal venue coverage.

## Phase 6 — tune until it earns repeat use

**Deliverable:** a versioned internal release and a candid go/stop report.

Run the workflow for at least two weeks with at least five distinct real funding decisions. They need not all produce transactions: a justified decision to wait, reduce the request or preserve a reserve counts. Do not move funds merely to meet a quota. If the natural cadence is lower, extend the observation period and record that as evidence about usefulness.

For every decision record: setup effort, data gaps, planning time, options considered, choice made, observed result, stale/incorrect alerts and manual work remaining. Tune the highest-cost recurring problem first. Feed fixes back into the affected phase and its regression checks.

Exit gate:

- The latest build completes the supported workflow without code edits or hand-edited state.
- No unresolved capital-loss-risk defect, false feasible classification, silent stale state or material reconciliation discrepancy remains in the supported scope.
- Recoverability and timing are measured on the final build. Repeated use yields a concrete benefit beyond a combined balance screen.
- The release includes setup instructions, supported-account/route coverage, limitations, private-data handling, a troubleshooting guide and known issues.

If we do not repeatedly use it ourselves, identify why before expanding integrations. If the only useful action would require us to lend money, explicitly revisit the thesis. More UI polish cannot pass that gate.

## Phase 7 — decide whether to sell

This starts only after the internal gates and explicit owner approval. No outreach or billing implementation belongs to Phases 1–6.

Review our decision log and whether a reachable buyer has the same recurring job. Recheck API/data permissions for the proposed commercial service, define customer terms under the repository's [proprietary license](../LICENSE), review third-party asset rights, and verify that operational costs and support can fit a subscription. These are release decisions; the repository license does not grant resale rights to upstream API data or third-party assets.

If approved, run the small external paid-pilot experiment in [the product decision](product.md#commercial-gates). Its $250/workspace/month price is a hypothesis. Build only the onboarding, isolation and access controls needed for the chosen pilot deployment before sharing accounts or hosting other users' data. External failure reports can send work back to earlier phases.

Outcomes: proceed with a narrow paid product, revise the customer/workflow on evidence, or stop Lane 1. None requires adding live credit, builder fees, a trading terminal or an execution bot. Cantina remains separate.

## Definition of done for every phase

- Runnable increment with a documented entry point; instructions describe only commands that exist.
- Acceptance evidence and meaningful checks for the changed behavior, including the failure cases relevant to that increment.
- README status, roadmap and supported coverage updated together. Unknowns and blockers recorded explicitly.
- Phase report: what changed, exact revision, checks, real vs replay evidence, unresolved issues, and next gate.
- Review the staged file list for local account data, secrets, research, unrelated repos and generated build outputs.
- Commit and push directly to `brohamgoham/hyprsonic` on `master`. No feature branch or PR required for this solo workflow.

Do not silently pass a gate by switching to a synthetic account, assuming settlement, claiming a private pledge is visible, or treating successful compilation as a completed user workflow.
