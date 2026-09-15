# Own-wallet trial runbook

**Planned for Phase 5. No live wallet test has been completed by writing this document.** Use this after the observation, reconciliation, planner and monitoring gates in [the roadmap](roadmap.md) pass.

The operator uses their own accounts. HyprSonic observes and explains; the operator chooses and performs any real transaction through the native wallet or venue. Never put private keys, seed phrases or credential values in a test report.

## Before the first run

Record locally:

- Build revision, adapter/rule versions, supported HL account mode and supported route.
- Account aliases and verified holding-account mappings; actual addresses and transaction references stay in private evidence.
- Source/destination chain and exact token identity, decimals and gas requirements.
- Test amount and total fee cap chosen by the operator; minimum reserves and destination margin buffer to preserve.
- Deadline/horizon, selected stress scenario and stop conditions. Stop on unexplained balances, stale required evidence or unsupported account rules.
- Evidence location, retention choice and redaction procedure. Verify the local configuration, secrets and journals are ignored by Git.

Account mode and route must be supported by the build being tested. A visible balance alone does not establish permission to withdraw or pledge it. Use existing activity where possible. Do not deliberately create a liquidation, disputed market or failed financial transfer to exercise a software branch.

## Trial sequence

| Case | Operator action | Required observation | Pass condition |
|---|---|---|---|
| Initial reconciliation | Compare the connected state with native account views at a matched time | Units, commitments, local collateral and source age | Differences explained within declared precision; no duplicated assets |
| Funding request | Enter destination amount, reserve policy and deadline | Alternatives, costs, buffers and dependencies | Every amount traces to evidence; assumptions labeled |
| Actual route | Choose a supported route and perform its steps manually | Source movement, in-flight state, actual destination credit | Confirmation comes from evidence, not the operator's click |
| Final reconciliation | Compare resulting accounts and fees | Actual input/output, fees, remaining source reserve and HL collateral | No unexplained residual or constraint breach |
| Rejected request | Enter an impossible amount or deadline; submit no transaction | Location and size of deficit or timing/coverage blocker | Explicit infeasible/undetermined result |
| Pending state | Observe ordinary real pending/redemption/transfer activity | Eligibility versus confirmed receipt | No premature contribution to free capital; unobserved cases marked pending |
| Observer interruption | Stop our local process or disconnect it, then resume | Stale state, gap detection, resynchronization | Previous feasible verdict is withdrawn until reconciled |
| Plan invalidation | Change the policy or observe normal external account activity | New revision, changed verdict, retained old evidence | Earlier decision history remains intact |

A read-only shadow replay of captured observations can exercise unusual delays or duplicate events. Label it **replay**, including every injected fault. Do not present it as an actual blocked payout or completed live transfer.

## Private decision record

Copy this template into the ignored local journal directory introduced in Phase 1. This file is only the blank public template.

```text
Decision ID:
Build / rule versions:
Started at / observation window:
Account aliases / supported mode:
Destination asset / amount / deadline:
Source reserve / destination buffer:
Scenario assumptions:
Evidence coverage and missing facts:
Plans offered / first blocker for each:
Chosen action (including wait or do nothing):
Operator's reason:
Private receipt/evidence references:
Predicted fees / actual fees / difference explained:
Predicted timing / actual timing / difference explained:
Final reconciliation and destination credit:
Stale periods / reconnect behavior:
Incorrect or noisy alerts:
Manual steps and time spent:
What HyprSonic changed about the decision:
Defects / severity / next action:
Outcome: pass / fail / pending / unsupported
Evidence kind: live / recorded replay / injected fault
```

## From trial to internal release

Maintain a private issue list ordered by impact: false feasible results and incorrect balances first; stale/recovery defects next; missing useful routes and workflow friction after that. Fix the smallest change that removes the recurring problem and replay the affected case.

Phase 6 requires at least two weeks and five natural real decisions, a reconciled completed route, no unresolved critical correctness defect, and operation without editing source or stored state. Extend the period if decisions are infrequent; manufactured activity is not a substitute for usefulness.

Before publishing a phase report, remove account addresses, identifying balances, transaction hashes, credential material and raw capture paths unless the owner has explicitly chosen to publish a particular item. Sanitized examples must be labeled as sanitized, with the transformations described. Inspect captures for identifiers that survive in URLs or nested fields.

Finish with a short owner assessment: Would I use this for the next real decision? What did it save me from manually checking? What useful alternative did it expose? What remains unsupported? Only after those answers and the roadmap gates does an external pilot become a decision to consider.
