//! Versioned, snapshot-only standard cross-margin constraints; no price forecast.
use crate::Decimal;
use serde::Serialize;
use std::cmp::Ordering;

pub const RULE_VERSION: &str = "hl-standard-cross-2026-09-18-v1";
#[derive(Clone, Debug, Serialize)]
pub struct MarginTier {
    pub lower_bound: Decimal,
    pub max_leverage: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct MarginSchedule {
    pub coin: String,
    pub tiers: Vec<MarginTier>,
    pub evidence_id: String,
}
#[derive(Clone, Debug)]
pub struct CrossPosition {
    pub notional: Decimal,
    pub leverage: u32,
    pub schedule: MarginSchedule,
}
#[derive(Clone, Debug, Serialize)]
pub struct MarginCheck {
    pub rule_version: String,
    pub equity: Decimal,
    pub initial_required: Decimal,
    pub maintenance_required: Decimal,
    pub transfer_required: Decimal,
    pub maintenance_buffer: Decimal,
    pub model_withdrawal_ceiling: Decimal,
    pub rounding: String,
}
pub trait ConstraintModel {
    fn evaluate(
        &self,
        equity: &Decimal,
        positions: &[CrossPosition],
    ) -> Result<MarginCheck, &'static str>;
}
pub struct StandardCross;
impl ConstraintModel for StandardCross {
    fn evaluate(
        &self,
        equity: &Decimal,
        positions: &[CrossPosition],
    ) -> Result<MarginCheck, &'static str> {
        let mut initial = Decimal::zero();
        let mut maintenance = Decimal::zero();
        let mut total = Decimal::zero();
        for p in positions {
            if p.notional.atoms() < 0 || p.leverage == 0 {
                return Err("invalid position risk inputs");
            }
            initial = initial.checked_add(&p.notional.divide_ceil(p.leverage, 6)?)?;
            maintenance =
                maintenance.checked_add(&maintenance_for(&p.notional, &p.schedule.tiers)?)?;
            total = total.checked_add(&p.notional)?;
        }
        let ten_percent = total.divide_ceil(10, 6)?;
        let transfer = if initial.compare(&ten_percent)? == Ordering::Greater {
            initial.clone()
        } else {
            ten_percent
        };
        let ceiling = equity.checked_sub(&transfer)?;
        Ok(MarginCheck {
            rule_version: RULE_VERSION.into(), equity: equity.clone(), initial_required: initial,
            maintenance_buffer: equity.checked_sub(&maintenance)?, maintenance_required: maintenance,
            transfer_required: transfer,
            model_withdrawal_ceiling: if ceiling.atoms() < 0 { Decimal::zero() } else { ceiling },
            rounding: "Each tier segment and each position initial margin rounds upward to 0.000001 reported USD; no silent tolerance.".into(),
        })
    }
}
/// Integrate piecewise maintenance rates; equivalent to tier deductions, with explicit conservative rounding.
pub fn maintenance_for(notional: &Decimal, tiers: &[MarginTier]) -> Result<Decimal, &'static str> {
    if notional.atoms() < 0 || tiers.is_empty() || tiers[0].lower_bound.atoms() != 0 {
        return Err("invalid margin table");
    }
    let mut total = Decimal::zero();
    for (i, tier) in tiers.iter().enumerate() {
        if tier.max_leverage == 0 || tier.max_leverage > 1000 || tier.lower_bound.atoms() < 0 {
            return Err("invalid tier leverage");
        }
        if let Some(next) = tiers.get(i + 1)
            && (next.lower_bound.compare(&tier.lower_bound)? != Ordering::Greater
                || next.max_leverage > tier.max_leverage)
        {
            return Err("unordered margin tiers");
        }
        if notional.compare(&tier.lower_bound)? != Ordering::Greater {
            continue;
        }
        let end = match tiers.get(i + 1) {
            Some(next) if next.lower_bound.compare(notional)? == Ordering::Less => {
                &next.lower_bound
            }
            _ => notional,
        };
        total = total.checked_add(
            &end.checked_sub(&tier.lower_bound)?
                .divide_ceil(tier.max_leverage * 2, 6)?,
        )?;
    }
    Ok(total)
}
