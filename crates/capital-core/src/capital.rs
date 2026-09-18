//! Evidence-backed capital projections. Views overlap; this module never sums venues.
use crate::margin::{ConstraintModel, CrossPosition, MarginCheck, StandardCross};
use crate::{AccountObservation, AssetId, Decimal, Fact, FactValue, Issue, ReadStatus};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const RULE_VERSION: &str = "capital-2026-09-18-v1";
#[derive(Clone, Debug, Serialize)]
pub struct ObservedActivity {
    pub event_id: String,
    pub at_ms: u64,
    pub kind: String,
    pub evidence_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Projection {
    pub amount: Option<Decimal>,
    pub basis: String,
    pub evidence: Vec<String>,
}
impl Projection {
    fn unknown(reason: &str) -> Self {
        Self {
            amount: None,
            basis: reason.into(),
            evidence: vec![],
        }
    }
    fn known(amount: Decimal, basis: &str, evidence: Vec<String>) -> Self {
        Self {
            amount: Some(amount),
            basis: basis.into(),
            evidence,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CapitalStates {
    pub settled: Projection,
    pub withdrawable: Projection,
    pub unrealized: Projection,
    pub pending_proceeds: Projection,
    pub pledged_reserved: Projection,
    pub delayed_blocked: Projection,
}
impl Default for CapitalStates {
    fn default() -> Self {
        Self {
            settled: Projection::unknown("Settlement not established for this claim."),
            withdrawable: Projection::unknown("Withdrawal eligibility is not established."),
            unrealized: Projection::unknown("No applicable unrealized valuation model."),
            pending_proceeds: Projection::unknown(
                "No complete settlement/transfer history; unknown is not zero.",
            ),
            pledged_reserved: Projection::unknown(
                "Private obligations and unobserved orders are unknown.",
            ),
            delayed_blocked: Projection::unknown(
                "Delay amount and release time are not established.",
            ),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CapitalClaim {
    pub id: String,
    pub account_alias: String,
    pub holder: String,
    pub asset: AssetId,
    pub collateral_domain: String,
    pub kind: String,
    pub observed_quantity: Projection,
    pub states: CapitalStates,
    pub local_reserved: Projection,
    pub after_local_reserves: Projection,
    pub funding_verdict: String,
    pub blockers: Vec<String>,
    pub annotations: Vec<String>,
    pub margin: Option<MarginCheck>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LedgerEntry {
    pub sequence: usize,
    pub kind: String,
    pub claim_id: Option<String>,
    pub explanation: String,
    pub evidence: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CapitalReport {
    pub rule_version: String,
    pub evaluated_at_ms: u64,
    pub observation_window_ms: Option<(u64, u64)>,
    pub max_age_ms: u64,
    pub max_skew_ms: u64,
    pub scope: String,
    pub funding_verdict: String,
    pub claims: Vec<CapitalClaim>,
    pub issues: Vec<Issue>,
    pub ledger: Vec<LedgerEntry>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Reservation {
    pub id: String,
    pub claim_id: String,
    pub amount: Decimal,
}
pub struct ReconcilePolicy {
    pub now_ms: u64,
    pub max_age_ms: u64,
    pub max_skew_ms: u64,
    pub reservations: Vec<Reservation>,
}
fn amount(f: &Fact) -> Result<Decimal, &'static str> {
    match &f.value {
        FactValue::Amount(n) => Ok(n.clone()),
        _ => Err("wrong amount type"),
    }
}
fn field<'a>(
    o: &'a AccountObservation,
    name: &str,
    asset: Option<&str>,
) -> Result<&'a Fact, &'static str> {
    let mut matches = o.facts.iter().filter(|f| {
        f.field == name && asset.is_none_or(|id| f.asset.as_ref().is_some_and(|a| a.id == id))
    });
    let f = matches.next().ok_or("required field missing")?;
    if matches.next().is_some() {
        return Err("duplicate field");
    }
    Ok(f)
}
fn number(
    o: &AccountObservation,
    name: &str,
    asset: Option<&str>,
) -> Result<Decimal, &'static str> {
    amount(field(o, name, asset)?)
}
fn string(o: &AccountObservation, name: &str, asset: Option<&str>) -> Result<String, &'static str> {
    match &field(o, name, asset)?.value {
        FactValue::Text(s) => Ok(s.clone()),
        _ => Err("wrong text type"),
    }
}
fn evidence(o: &AccountObservation) -> Vec<String> {
    o.evidence.iter().map(|e| e.id.clone()).collect()
}
fn from_fact(f: &Fact, basis: &str) -> Result<Projection, &'static str> {
    Ok(Projection::known(
        amount(f)?,
        basis,
        vec![f.evidence_id.clone()],
    ))
}
fn claim(
    o: &AccountObservation,
    f: &Fact,
    kind: &str,
    domain: &str,
) -> Result<CapitalClaim, &'static str> {
    let asset = f.asset.clone().ok_or("missing asset identity")?;
    Ok(CapitalClaim {
        id: format!(
            "{}:{}:{}:{}:{}",
            o.account.venue,
            o.account.address.as_str(),
            domain,
            asset.network,
            asset.id
        ),
        account_alias: o.alias.clone(),
        holder: o.account.address.as_str().into(),
        asset,
        collateral_domain: domain.into(),
        kind: kind.into(),
        observed_quantity: from_fact(
            f,
            "Observed holding; not automatically free funding capacity.",
        )?,
        states: CapitalStates::default(),
        local_reserved: Projection::known(
            Decimal::zero(),
            "No reservations in this supplied local policy; external obligations remain unknown.",
            vec![],
        ),
        after_local_reserves: Projection::unknown("No supported availability ceiling."),
        funding_verdict: "UNDETERMINED".into(),
        blockers: vec![
            "ACCOUNT_CONTROL_AND_EXTERNAL_OBLIGATIONS_UNKNOWN".into(),
            "TRANSFER_ROUTE_AND_DESTINATION_CREDIT_UNVERIFIED".into(),
        ],
        annotations: vec![],
        margin: None,
    })
}

pub fn reconcile(accounts: &[AccountObservation], policy: &ReconcilePolicy) -> CapitalReport {
    let mut out = CapitalReport {
        rule_version: RULE_VERSION.into(), evaluated_at_ms: policy.now_ms,
        observation_window_ms: None, max_age_ms: policy.max_age_ms, max_skew_ms: policy.max_skew_ms,
        scope: "Configured holdings only. Six views overlap. No portfolio cash total, global atomic snapshot, inferred receipts, or executable funding verdict.".into(),
        funding_verdict: "UNDETERMINED".into(), claims: vec![], issues: vec![], ledger: vec![],
    };
    if accounts.is_empty() || policy.max_age_ms == 0 || policy.max_skew_ms == 0 {
        issue(
            &mut out,
            "INVALID_SCOPE",
            "portfolio",
            "Accounts and positive freshness/skew bounds are required.",
        );
    }
    let times: Vec<_> = accounts
        .iter()
        .flat_map(|o| &o.evidence)
        .map(|e| e.source_time_ms.unwrap_or(e.received_at_ms))
        .collect();
    if let (Some(first), Some(last)) = (times.iter().min(), times.iter().max()) {
        out.observation_window_ms = Some((*first, *last));
        if last - first > policy.max_skew_ms {
            issue(
                &mut out,
                "SNAPSHOT_SKEW",
                "portfolio",
                "Sources exceed the allowed observation window; they cannot form one coherent funding snapshot.",
            );
        }
    }
    let mut identities = BTreeSet::new();
    let mut all_evidence = BTreeSet::new();
    for o in accounts {
        let networks: BTreeSet<_> = o
            .facts
            .iter()
            .filter_map(|f| f.asset.as_ref().map(|a| a.network.as_str()))
            .collect();
        let identity = (
            o.account.venue.clone(),
            o.account.address.as_str().to_owned(),
            networks,
        );
        if !identities.insert(identity) {
            issue(
                &mut out,
                "DUPLICATE_ACCOUNT",
                &o.alias,
                "The same account snapshot was supplied twice; second copy was excluded.",
            );
            continue;
        }
        let first_claim = out.claims.len();
        let evidence_ids: BTreeSet<_> = o.evidence.iter().map(|e| e.id.as_str()).collect();
        let mut invalid = !matches!(o.read_status, ReadStatus::Complete)
            || !o.issues.is_empty()
            || o.evidence.is_empty();
        out.issues.extend(o.issues.clone());
        for e in &o.evidence {
            if !all_evidence.insert(e.id.clone()) {
                invalid = true;
                issue(
                    &mut out,
                    "DUPLICATE_EVIDENCE",
                    &o.alias,
                    "Evidence identifiers must be unique in this run.",
                );
            }
            let t = e.source_time_ms.unwrap_or(e.received_at_ms);
            if t > policy.now_ms.saturating_add(5000)
                || policy.now_ms.saturating_sub(t) > policy.max_age_ms
            {
                invalid = true;
                issue(
                    &mut out,
                    "STALE_OR_FUTURE_EVIDENCE",
                    &o.alias,
                    "Source evidence is outside the freshness policy.",
                );
            }
        }
        let mut facts = BTreeSet::new();
        for f in &o.facts {
            if !evidence_ids.contains(f.evidence_id.as_str()) || !facts.insert((&f.field, &f.asset))
            {
                invalid = true;
                issue(
                    &mut out,
                    "CONFLICTING_FACTS",
                    &o.alias,
                    "Duplicate field or missing evidence reference; no availability may be inferred.",
                );
            }
        }
        let result = match o.account.venue.as_str() {
            "evm" => wallet(o, &mut out),
            "hyperliquid" => hyperliquid(o, &mut out),
            "polymarket" => predictions(o, &mut out),
            _ => Err("unsupported venue"),
        };
        if let Err(reason) = result {
            invalid = true;
            issue(&mut out, "RECONCILIATION_GAP", &o.alias, reason);
            if o.account.venue == "hyperliquid"
                && let Ok(f) = field(o, "default_dex.reported.accountValue", None)
                && let Ok(mut c) = claim(o, f, "unresolved_hl_collateral", "unresolved-perps")
            {
                c.observed_quantity.basis =
                    "Raw default-DEX equity view; no independent spendable claim established."
                        .into();
                out.claims.push(c);
            }
        }
        if invalid {
            issue(
                &mut out,
                "ACCOUNT_INCOMPLETE",
                &o.alias,
                "Preserved observed quantities; capital projections require complete, fresh, consistent evidence.",
            );
            for c in &mut out.claims[first_claim..] {
                invalidate(c, "ACCOUNT_EVIDENCE_INCOMPLETE");
            }
        }
        ledger(
            &mut out,
            "observation",
            None,
            &format!(
                "{}: {} facts, {} responses; source limitations retained in account report.",
                o.alias,
                o.facts.len(),
                o.evidence.len()
            ),
            evidence(o),
        );
        let mut events = BTreeMap::new();
        let activity_issue_start = out.issues.len();
        let watermark = o.evidence.iter().filter_map(|e| e.source_time_ms).max();
        for event in &o.activities {
            if !evidence_ids.contains(event.evidence_id.as_str())
                || event.at_ms > policy.now_ms.saturating_add(5000)
            {
                issue(
                    &mut out,
                    "INVALID_ACTIVITY",
                    &o.alias,
                    "Activity lacks source evidence or has a future timestamp.",
                );
                continue;
            }
            let identity = (event.at_ms, event.kind.clone());
            if let Some(previous) = events.insert(event.event_id.clone(), identity.clone()) {
                if previous != identity {
                    issue(
                        &mut out,
                        "CONFLICTING_EVENT",
                        &o.alias,
                        "Same event identifier has conflicting content.",
                    );
                } else {
                    ledger(
                        &mut out,
                        "duplicate_event",
                        None,
                        "Duplicate historical event excluded; no balance adjustment.",
                        vec![event.evidence_id.clone()],
                    );
                }
                continue;
            }
            if watermark.is_some_and(|t| event.at_ms > t) {
                issue(
                    &mut out,
                    "EVENT_AFTER_SNAPSHOT",
                    &o.alias,
                    "Ledger activity is newer than the account snapshot; recollect before using capacity.",
                );
                for c in &mut out.claims[first_claim..] {
                    invalidate(c, "EVENT_AFTER_SNAPSHOT");
                }
            }
            ledger(
                &mut out,
                "venue_activity",
                None,
                &format!(
                    "{} event={} kind={} time={}; historical activity only. Snapshot remains authoritative; do not add this event as another deposit or infer destination credit.",
                    o.alias, event.event_id, event.kind, event.at_ms
                ),
                vec![event.evidence_id.clone()],
            );
        }
        if out.issues.len() > activity_issue_start {
            for c in &mut out.claims[first_claim..] {
                invalidate(c, "ACTIVITY_RECONCILIATION_GAP");
            }
        }
    }
    let mut claim_ids = BTreeSet::new();
    let duplicates: Vec<_> = out
        .claims
        .iter()
        .filter_map(|c| {
            if claim_ids.insert(c.id.clone()) {
                None
            } else {
                Some(c.id.clone())
            }
        })
        .collect();
    for id in duplicates {
        out.claims.retain(|c| c.id != id);
        issue(
            &mut out,
            "DUPLICATE_CLAIM",
            &id,
            "Conflicting representations of one holding were excluded rather than added.",
        );
    }
    // A matching holding wallet links the views, but does not create another cash claim.
    for c in &mut out.claims {
        if c.kind == "wallet_balance"
            && c.asset.network == "evm:137"
            && accounts
                .iter()
                .any(|o| o.account.venue == "polymarket" && o.account.address.as_str() == c.holder)
        {
            c.annotations.push("Same holder as the configured Polymarket account; this is the single onchain balance view, not additional venue cash.".into());
            c.blockers.push("POLYMARKET_OPEN_ORDERS_UNKNOWN".into());
        }
    }
    if out.issues.iter().any(|i| {
        matches!(
            i.code.as_str(),
            "SNAPSHOT_SKEW" | "DUPLICATE_ACCOUNT" | "DUPLICATE_EVIDENCE"
        )
    }) {
        for c in &mut out.claims {
            invalidate(c, "SNAPSHOT_INCONSISTENT");
        }
    }
    apply_reservations(&mut out, &policy.reservations);
    for c in out.claims.clone() {
        for (name, p) in [
            ("settled", &c.states.settled),
            ("withdrawable", &c.states.withdrawable),
            ("unrealized", &c.states.unrealized),
            ("pending_proceeds", &c.states.pending_proceeds),
            ("pledged_reserved", &c.states.pledged_reserved),
            ("delayed_blocked", &c.states.delayed_blocked),
        ] {
            ledger(
                &mut out,
                "classification",
                Some(c.id.clone()),
                &format!(
                    "{name}={}: {}",
                    p.amount
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or("UNKNOWN".into()),
                    p.basis
                ),
                p.evidence.clone(),
            );
        }
    }
    out
}
fn invalidate(c: &mut CapitalClaim, reason: &str) {
    c.states = CapitalStates::default();
    c.margin = None;
    c.after_local_reserves = Projection::unknown(reason);
    c.blockers.push(reason.into());
}
fn issue(out: &mut CapitalReport, code: &str, scope: &str, detail: &str) {
    out.issues.push(Issue {
        code: code.into(),
        scope: scope.into(),
        detail: detail.into(),
    });
}
fn ledger(
    out: &mut CapitalReport,
    kind: &str,
    claim_id: Option<String>,
    explanation: &str,
    evidence: Vec<String>,
) {
    out.ledger.push(LedgerEntry {
        sequence: out.ledger.len() + 1,
        kind: kind.into(),
        claim_id,
        explanation: explanation.into(),
        evidence,
    });
}
fn wallet(o: &AccountObservation, out: &mut CapitalReport) -> Result<(), &'static str> {
    let finalized = string(o, "wallet.block_policy", None)? == "finalized";
    for f in o.facts.iter().filter(|f| {
        f.field.starts_with("wallet.token_balance.") || f.field == "wallet.native_balance"
    }) {
        if amount(f)?.atoms() < 0 {
            return Err("negative onchain holding");
        }
        let mut c = claim(o, f, "wallet_balance", "onchain")?;
        if finalized {
            c.states.settled = from_fact(
                f,
                "Balance at RPC-reported finalized block, rechecked canonical during collection; RPC trust remains.",
            )?;
        }
        c.blockers
            .push("GAS_SPEND_AUTHORITY_AND_COMMITMENTS_UNVERIFIED".into());
        c.annotations.push("Balance snapshots already include credited transfers. Never add activity amounts to this holding.".into());
        out.claims.push(c);
    }
    Ok(())
}
fn hyperliquid(o: &AccountObservation, out: &mut CapitalReport) -> Result<(), &'static str> {
    let supported = o.account_mode.as_deref() == Some("disabled")
        && matches!(
            string(o, "account.role", None)?.as_str(),
            "user" | "subAccount"
        );
    if !supported {
        issue(
            out,
            "HL_MODE_UNSUPPORTED",
            &o.alias,
            "Only explicit disabled/standard user or subaccount mode has a local margin model. default, unified, portfolio, DEX abstraction and vaults remain observed-only.",
        );
    }
    // Unified and portfolio modes: use spot holdings only. Per-DEX equity is not an independent claim.
    for f in o.facts.iter().filter(|f| f.field == "spot.reported.total") {
        let mut c = claim(o, f, "hl_spot", "spot-or-shared-collateral")?;
        if supported {
            let hold = field(o, "spot.reported.hold", Some(&c.asset.id))?;
            let total = amount(f)?;
            let reserved = amount(hold)?;
            if total.atoms() < 0
                || reserved.atoms() < 0
                || reserved.compare(&total)? == Ordering::Greater
            {
                return Err("spot hold outside balance");
            }
            c.states.settled = from_fact(
                f,
                "Venue credited spot total in explicit standard mode; holds overlap this balance.",
            )?;
            c.states.pledged_reserved = from_fact(
                hold,
                "Venue-reported spot hold only; not a complete inventory of private pledges.",
            )?;
            c.states.delayed_blocked = c.states.pledged_reserved.clone();
        } else {
            c.blockers.push("HL_MODE_UNSUPPORTED".into());
        }
        out.claims.push(c);
    }
    if !supported {
        if !out.claims.iter().any(|c| c.account_alias == o.alias) {
            let f = field(o, "default_dex.reported.accountValue", None)?;
            let mut c = claim(o, f, "unresolved_hl_collateral", "unresolved-or-shared")?;
            c.observed_quantity = Projection::unknown(
                "Raw per-DEX equity cannot establish an independent holding in this account mode; inspect original account facts.",
            );
            c.blockers.push("HL_MODE_UNSUPPORTED".into());
            out.claims.push(c);
        }
        return Ok(());
    }
    let equity_fact = field(o, "default_dex.reported.accountValue", None)?;
    let mut c = claim(
        o,
        equity_fact,
        "hl_cross_collateral",
        "standard-default-perps",
    )?;
    let mut positions = vec![];
    let mut pnl = Decimal::zero();
    for size in o.facts.iter().filter(|f| f.field == "position.szi") {
        let coin = &size.asset.as_ref().ok_or("position identity missing")?.id;
        if string(o, "position.margin_mode", Some(coin))? != "cross" {
            return Err("isolated positions require a separate collateral/risk model");
        }
        let lev = number(o, "position.leverage", Some(coin))?;
        if lev.scale() != 0 || lev.atoms() <= 0 || lev.atoms() > 1000 {
            return Err("invalid position leverage");
        }
        let usd_id = format!("reported-USD:{coin}");
        pnl = pnl.checked_add(&number(o, "position.unrealizedPnl", Some(&usd_id))?)?;
        let schedule = o
            .margin_schedules
            .iter()
            .find(|s| &s.coin == coin)
            .ok_or("missing live margin table")?
            .clone();
        if !o.evidence.iter().any(|e| e.id == schedule.evidence_id) {
            return Err("margin schedule evidence missing");
        }
        positions.push(CrossPosition {
            notional: number(o, "position.positionValue", Some(&usd_id))?,
            leverage: lev.atoms() as u32,
            schedule,
        });
    }
    let equity = amount(equity_fact)?;
    let total_notional = positions
        .iter()
        .try_fold(Decimal::zero(), |sum, p| sum.checked_add(&p.notional))?;
    if total_notional.compare(&number(o, "default_dex.reported.totalNtlPos", None)?)?
        != Ordering::Equal
    {
        return Err("position notionals disagree with account summary");
    }
    let risk = StandardCross.evaluate(&equity, &positions)?;
    let reported_mm = number(o, "default_dex.reported.crossMaintenanceMarginUsed", None)?;
    let rounding_tolerance = Decimal::from_atoms(
        positions
            .iter()
            .map(|p| p.schedule.tiers.len() as i128)
            .sum(),
        6,
    )?;
    let diff = risk.maintenance_required.checked_sub(&reported_mm)?;
    if diff.atoms() < 0 || diff.compare(&rounding_tolerance)? == Ordering::Greater {
        return Err(
            "computed maintenance disagrees with venue beyond conservative per-tier rounding",
        );
    }
    c.states.settled = Projection::known(
        equity.checked_sub(&pnl)?,
        "Inferred collateral equity excluding current position unrealized PnL; default DEX standard cross-only scope.",
        evidence(o),
    );
    c.states.unrealized = Projection::known(
        pnl,
        "Sum of position unrealized PnL in this collateral pool; included in equity, never added again.",
        evidence(o),
    );
    c.states.pledged_reserved = Projection::known(
        risk.initial_required.clone(),
        "Computed initial requirement for existing positions only. Open-order/private commitments remain separate unknowns.",
        evidence(o),
    );
    let withdrawal = field(o, "default_dex.reported.withdrawable", None)?;
    let n = amount(withdrawal)?;
    if n.atoms() < 0 || n.compare(&risk.model_withdrawal_ceiling)? == Ordering::Greater {
        return Err("venue withdrawal exceeds independently computed transfer-margin ceiling");
    }
    c.states.withdrawable = from_fact(
        withdrawal,
        "Venue-reported withdrawal ceiling checked against transfer-margin floor; not a completed transfer or funding approval.",
    )?;
    if number(o, "default_dex.open_order_count", None)?.atoms() != 0 {
        c.blockers.push("OPEN_ORDER_MARGIN_NOT_MODELED".into());
    } else {
        c.after_local_reserves = c.states.withdrawable.clone();
    }
    c.states.delayed_blocked = Projection::known(
        equity.checked_sub(&n)?,
        "Equity not reported withdrawable; overlaps margin/other restrictions and is not a separate asset.",
        evidence(o),
    );
    if risk.maintenance_buffer.atoms() < 0 {
        c.blockers.push("HL_MARGIN_UNMET".into());
        c.after_local_reserves = Projection::unknown("Local maintenance margin is unmet.");
        issue(
            out,
            "HL_MARGIN_UNMET",
            &o.alias,
            "Local cross equity is below maintenance. Holdings at other accounts cannot cure this snapshot shortfall.",
        );
    }
    c.annotations.push(format!(
        "Maintenance reconciliation difference={} reported USD; rounding bound={}.",
        diff, rounding_tolerance
    ));
    c.margin = Some(risk);
    out.claims.push(c);
    Ok(())
}
fn predictions(o: &AccountObservation, out: &mut CapitalReport) -> Result<(), &'static str> {
    for f in o.facts.iter().filter(|f| f.field == "position.size") {
        let mut c = claim(o, f, "outcome_tokens", "prediction-position")?;
        let id = c.asset.id.clone();
        let redeemable = field(o, "position.redeemable_hint", Some(&id))?;
        c.blockers
            .push("OUTCOME_TOKENS_ARE_NOT_CREDITED_COLLATERAL".into());
        c.blockers
            .push("REDEMPTION_RECEIPT_AND_PAYOUT_UNVERIFIED".into());
        c.annotations.push(format!(
            "redeemable hint={:?}; neither this flag nor currentValue is cash.",
            redeemable.value
        ));
        let condition = id.split(':').next().ok_or("missing condition")?;
        match string(o, "market.lifecycle", Some(condition)) {
            Ok(state) => {
                c.annotations.push(format!("Market service lifecycle={state}; onchain payout and credited collateral remain unverified."));
                if state != "resolved" {
                    c.blockers.push("RESOLUTION_OR_CHALLENGE_PENDING".into());
                }
            }
            Err(_) => {
                c.blockers.push("MARKET_LIFECYCLE_UNKNOWN".into());
            }
        }
        c.states.pending_proceeds = Projection::unknown(
            "Outcome quantity is known; payout amount and completion time need verified resolution/redemption evidence. No cash contribution.",
        );
        c.states.delayed_blocked = Projection::known(
            amount(f)?,
            "Outcome token units excluded from direct cash funding until conversion and destination credit; no timing guarantee.",
            vec![f.evidence_id.clone()],
        );
        out.claims.push(c);
    }
    Ok(())
}
fn apply_reservations(out: &mut CapitalReport, reservations: &[Reservation]) {
    let mut ids = BTreeMap::new();
    let mut sums: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut bad_claims = BTreeSet::new();
    for r in reservations {
        let previous = ids.insert(r.id.clone(), r.claim_id.clone());
        let valid = !r.id.is_empty() && previous.is_none() && r.amount.atoms() > 0;
        if let Some(id) = previous {
            bad_claims.insert(id);
        }
        if !valid {
            bad_claims.insert(r.claim_id.clone());
            issue(
                out,
                "INVALID_RESERVATION",
                &r.id,
                "Reservation identifiers must be unique and amounts positive.",
            );
            continue;
        }
        let sum = sums.entry(r.claim_id.clone()).or_insert_with(Decimal::zero);
        match sum.checked_add(&r.amount) {
            Ok(n) => *sum = n,
            Err(_) => {
                bad_claims.insert(r.claim_id.clone());
                issue(
                    out,
                    "RESERVATION_OVERFLOW",
                    &r.id,
                    "Reservation amount exceeds supported precision.",
                );
            }
        }
        ledger(
            out,
            "user_policy",
            Some(r.claim_id.clone()),
            &format!(
                "Local reservation {} amount={}; this does not lock assets at a venue.",
                r.id, r.amount
            ),
            vec![],
        );
    }
    for (id, amount) in sums {
        let Some(index) = out.claims.iter().position(|c| c.id == id) else {
            issue(
                out,
                "RESERVATION_CLAIM_MISSING",
                &id,
                "Reservation references no observed holding.",
            );
            continue;
        };
        let c = &mut out.claims[index];
        c.local_reserved = Projection::known(
            amount.clone(),
            "Sum of supplied local reservations; excludes venue holds and private pledges.",
            vec![],
        );
        // Reservations consume observed ceilings, not pending/redeemable positions.
        // Spot holds already reserve the same asset and must be subtracted first.
        let ceiling = c.after_local_reserves.amount.clone().or_else(|| {
            if c.kind == "hl_spot" {
                c.states
                    .settled
                    .amount
                    .as_ref()?
                    .checked_sub(c.states.pledged_reserved.amount.as_ref()?)
                    .ok()
            } else if c.kind == "wallet_balance" {
                c.states.settled.amount.clone()
            } else {
                None
            }
        });
        if let Some(ceiling) = ceiling {
            match ceiling.checked_sub(&amount) {
                Ok(left) if left.atoms() >= 0 => {
                    if c.after_local_reserves.amount.is_some() {
                        c.after_local_reserves.amount = Some(left);
                        c.after_local_reserves
                            .basis
                            .push_str(" Less user-declared local reservations.");
                    }
                }
                _ => {
                    bad_claims.insert(id.clone());
                    issue(
                        out,
                        "DOUBLE_PLEDGE_OR_OVERRESERVE",
                        &id,
                        "Local reservations exceed the available observed ceiling; no plan may consume this claim.",
                    );
                }
            }
        } else {
            bad_claims.insert(id.clone());
            issue(
                out,
                "RESERVATION_CAPACITY_UNKNOWN",
                &id,
                "Cannot validate a reservation against missing/unsupported capital.",
            );
        }
    }
    for c in &mut out.claims {
        if bad_claims.contains(&c.id) {
            c.after_local_reserves =
                Projection::unknown("Invalid or conflicting local reservations.");
            c.blockers.push("RESERVATION_CONFLICT".into());
        }
    }
}
