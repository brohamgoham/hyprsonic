use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub type Money = i64; // Integer synthetic USD cents. Never token conversion or floating point.

pub fn dollars(cents: Money) -> String {
    let n = cents.unsigned_abs();
    format!(
        "{}${}.{:02}",
        if cents < 0 { "-" } else { "" },
        n / 100,
        n % 100
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub id: String,
    pub synthetic: bool,
    pub horizon: u32,
    pub action: Action,
    pub capital: Capital,
    pub rules: Rules,
    pub price_move: PriceMove,
    pub quote: Quote,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub description: String,
    pub at: u32,
    pub initial_margin_cents: Money,
    pub maintenance_cents: Money,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capital {
    pub wallet_withdrawable_cents: Money,
    pub wallet_already_pledged_cents: Money,
    pub hl_settled_cents: Money,
    pub poly_withdrawable_cents: Money,
    pub poly_pending_cents: Money,
    pub kalshi_pending_cents: Money,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub hl_existing_maintenance_cents: Money,
    pub poly_settlement_at: u32,
    pub poly_challenge_at: u32,
    pub poly_delayed_settlement_at: u32,
    pub kalshi_close_at: u32,
    pub kalshi_determination_at: u32,
    pub kalshi_settlement_at: u32,
    pub kalshi_commercial_access_met: bool,
    pub transfer_ticks: u32,
    pub wallet_transfer_cents: Money,
    pub wallet_fee_cents: Money,
    pub withdraw_transfer_cents: Money,
    pub withdraw_fee_cents: Money,
    pub payout_sweep_fee_cents: Money,
    pub reduce_fee_cents: Money,
    pub reduce_remaining_exposure_bps: i64,
    pub reduce_margin_relief_cents: Money,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PriceMove {
    pub at: u32,
    pub existing_position_loss_cents: Money,
    pub proposed_position_loss_cents: Money,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    pub id: String,
    pub synthetic: bool,
    pub principal_cents: Money,
    pub upfront_fee_cents: Money,
    pub interest_bps_for_horizon: i64,
    pub max_ltv_bps: i64,
    pub fund_at: u32,
    pub expires_at: u32,
    pub matures_at: u32,
    pub collateral_lots: Vec<String>,
}

impl Fixture {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let f: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        f.validate()?;
        Ok(f)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.synthetic && self.quote.synthetic,
            "Phase 0 accepts synthetic fixtures and quotes only"
        );
        ensure!(
            self.quote.id.starts_with("SYNTHETIC-"),
            "quote ID must be explicitly SYNTHETIC- prefixed"
        );
        ensure!(
            !self.rules.kalshi_commercial_access_met,
            "Kalshi commercial access must remain explicitly unmet"
        );
        ensure!(
            (1..=100).contains(&self.horizon),
            "horizon must be 1..100 synthetic ticks"
        );
        // Bound every numeric input before arithmetic: all products/sums remain within i64.
        fn bounded(v: &serde_json::Value) -> bool {
            match v {
                serde_json::Value::Number(n) => {
                    n.as_i64().is_some_and(|n| (0..=1_000_000_000).contains(&n))
                }
                serde_json::Value::Object(o) => o.values().all(bounded),
                serde_json::Value::Array(a) => a.iter().all(bounded),
                _ => true,
            }
        }
        ensure!(
            bounded(&serde_json::to_value(self)?),
            "inputs must be nonnegative bounded integers (maximum 1e9)"
        );
        ensure!(
            self.action.at > 0
                && self.action.at < self.price_move.at
                && self.price_move.at <= self.horizon,
            "action must precede price move within horizon"
        );
        ensure!(
            self.action.initial_margin_cents >= self.action.maintenance_cents
                && self.action.maintenance_cents > 0,
            "invalid action margin requirements"
        );
        ensure!(
            self.rules.reduce_margin_relief_cents < self.action.maintenance_cents
                && self.rules.reduce_margin_relief_cents < self.rules.hl_existing_maintenance_cents,
            "reduction cannot erase margin requirements"
        );
        ensure!(
            (1..=100).contains(&self.rules.transfer_ticks),
            "transfer delay must be 1..100 ticks"
        );
        ensure!(
            self.rules.poly_challenge_at < self.rules.poly_settlement_at
                && self.rules.poly_settlement_at < self.rules.poly_delayed_settlement_at,
            "invalid Polymarket lifecycle ordering"
        );
        ensure!(
            self.rules.kalshi_close_at < self.rules.kalshi_determination_at
                && self.rules.kalshi_determination_at < self.rules.kalshi_settlement_at,
            "Kalshi close, determination and settlement must be distinct and ordered"
        );
        ensure!(
            (0..=10_000).contains(&self.rules.reduce_remaining_exposure_bps),
            "invalid reduction fraction"
        );
        ensure!(
            (1..=10_000).contains(&self.quote.max_ltv_bps)
                && self.quote.interest_bps_for_horizon <= 10_000,
            "invalid synthetic financing terms"
        );
        ensure!(
            self.quote.principal_cents > self.quote.upfront_fee_cents,
            "quote fee consumes principal"
        );
        ensure!(
            self.quote.fund_at > 0 && self.quote.matures_at > self.horizon,
            "funding starts after reservation; loan maturity must be beyond modeled horizon"
        );
        ensure!(
            !self.quote.collateral_lots.is_empty() && self.quote.collateral_lots.len() <= 8,
            "invalid collateral lot count"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue {
    Wallet,
    Hyperliquid,
    Polymarket,
    Kalshi,
    InTransit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum CapitalState {
    SettledLocked,
    Withdrawable,
    Reserved(String),
    Pledged(String),
    PendingProceeds,
    Delayed(String),
    InTransit,
    Blocked(String),
}

#[derive(Clone, Debug, Serialize)]
pub struct Lot {
    pub id: String,
    pub venue: Venue,
    pub cents: Money,
    pub state: CapitalState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Wallet,
    Withdraw,
    Reduce,
    Finance,
}
impl Plan {
    pub const ALL: [Self; 4] = [Self::Wallet, Self::Withdraw, Self::Reduce, Self::Finance];
    pub fn name(self) -> &'static str {
        match self {
            Self::Wallet => "wallet",
            Self::Withdraw => "withdraw",
            Self::Reduce => "reduce",
            Self::Finance => "finance",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == s)
            .ok_or_else(|| anyhow::anyhow!("unknown plan: {s}"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    OnTime,
    Delayed,
}
impl Scenario {
    pub fn name(self) -> &'static str {
        match self {
            Self::OnTime => "on-time",
            Self::Delayed => "delayed",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "on-time" => Ok(Self::OnTime),
            "delayed" => Ok(Self::Delayed),
            _ => anyhow::bail!("unknown scenario: {s}"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CapitalView {
    pub venue: Venue,
    pub settled_cents: Money,
    pub withdrawable_cents: Money,
    pub unrealized_cents: Money,
    pub pending_proceeds_cents: Money,
    pub pledged_reserved_cents: Money,
    pub delayed_blocked_cents: Money,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub capital: Vec<CapitalView>,
    pub lots: Vec<Lot>,
    pub aggregate_equity_cents: Money,
    pub loan_liability_cents: Money,
    pub hl_local_equity_cents: Money,
    pub hl_required_cents: Money,
    pub total_cost_cents: Money,
    pub action_executed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LedgerEvent {
    pub sequence: usize,
    pub tick: u32,
    pub code: String,
    pub explanation: String,
    pub state: Snapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub tick: u32,
    pub code: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub fixture_id: String,
    pub fixture: Fixture,
    pub plan: Plan,
    pub scenario: Scenario,
    pub injected_step_failure: bool,
    pub feasible: bool,
    pub failures: Vec<Failure>,
    pub ledger: Vec<LedgerEvent>,
    pub final_state: Snapshot,
}
