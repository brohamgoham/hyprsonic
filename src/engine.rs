use crate::model::*;
use anyhow::{Result, ensure};

struct Transfer {
    lot: String,
    arrives: u32,
}

pub struct Engine {
    f: Fixture,
    plan: Plan,
    scenario: Scenario,
    break_step: bool,
    tick: u32,
    lots: Vec<Lot>,
    transfers: Vec<Transfer>,
    loan: Money,
    costs: Money,
    unrealized: Money,
    reduced: bool,
    action: bool,
    finance_ready: bool,
    opening_equity: Money,
    failures: Vec<Failure>,
    ledger: Vec<LedgerEvent>,
}

impl Engine {
    pub fn new(f: Fixture, plan: Plan, scenario: Scenario, break_step: bool) -> Result<Self> {
        f.validate()?;
        let c = &f.capital;
        let specs = [
            (
                "wallet-ready",
                Venue::Wallet,
                c.wallet_withdrawable_cents,
                CapitalState::Withdrawable,
            ),
            (
                "wallet-pledged",
                Venue::Wallet,
                c.wallet_already_pledged_cents,
                CapitalState::Pledged("existing-obligation-no-loan".into()),
            ),
            (
                "hl-margin",
                Venue::Hyperliquid,
                c.hl_settled_cents,
                CapitalState::SettledLocked,
            ),
            (
                "poly-ready",
                Venue::Polymarket,
                c.poly_withdrawable_cents,
                CapitalState::Withdrawable,
            ),
            (
                "poly-payout",
                Venue::Polymarket,
                c.poly_pending_cents,
                CapitalState::PendingProceeds,
            ),
            (
                "kalshi-payout",
                Venue::Kalshi,
                c.kalshi_pending_cents,
                CapitalState::PendingProceeds,
            ),
        ];
        let lots: Vec<_> = specs
            .into_iter()
            .map(|(id, venue, cents, state)| Lot {
                id: id.into(),
                venue,
                cents,
                state,
            })
            .collect();
        let opening_equity = lots.iter().map(|l| l.cents).sum();
        Ok(Self {
            f,
            plan,
            scenario,
            break_step,
            tick: 0,
            lots,
            transfers: vec![],
            loan: 0,
            costs: 0,
            unrealized: 0,
            reduced: false,
            action: false,
            finance_ready: false,
            opening_equity,
            failures: vec![],
            ledger: vec![],
        })
    }

    fn lot_index(&self, id: &str) -> Result<usize> {
        self.lots
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown capital lot {id}"))
    }

    pub fn pledge(&mut self, id: &str, claim: &str) -> Result<()> {
        let i = self.lot_index(id)?;
        ensure!(
            self.lots[i].state == CapitalState::Withdrawable && self.lots[i].cents > 0,
            "DOUBLE_PLEDGE_OR_UNAVAILABLE: {id} is {:?}; it cannot back another commitment",
            self.lots[i].state
        );
        self.lots[i].state = CapitalState::Pledged(claim.into());
        self.record("COLLATERAL_PLEDGED", format!("{id} exclusively pledged to {claim}; unavailable for withdrawal or another plan step"));
        Ok(())
    }

    fn relief(&self) -> Money {
        if self.reduced {
            self.f.rules.reduce_margin_relief_cents
        } else {
            0
        }
    }
    fn maintenance(&self) -> Money {
        (if self.action {
            self.f.action.maintenance_cents
        } else {
            self.f.rules.hl_existing_maintenance_cents
        }) - self.relief()
    }
    fn hl_local(&self) -> Money {
        self.lots
            .iter()
            .filter(|l| {
                l.venue == Venue::Hyperliquid
                    && matches!(
                        l.state,
                        CapitalState::SettledLocked | CapitalState::Withdrawable
                    )
            })
            .map(|l| l.cents)
            .sum::<Money>()
            + self.unrealized
    }

    fn snapshot(&self) -> Snapshot {
        let capital = [
            Venue::Wallet,
            Venue::Hyperliquid,
            Venue::Polymarket,
            Venue::Kalshi,
            Venue::InTransit,
        ]
        .into_iter()
        .map(|venue| {
            let mut v = CapitalView {
                venue,
                settled_cents: 0,
                withdrawable_cents: 0,
                unrealized_cents: if venue == Venue::Hyperliquid {
                    self.unrealized
                } else {
                    0
                },
                pending_proceeds_cents: 0,
                pledged_reserved_cents: 0,
                delayed_blocked_cents: 0,
            };
            for l in self.lots.iter().filter(|l| l.venue == venue) {
                match &l.state {
                    CapitalState::SettledLocked => v.settled_cents += l.cents,
                    CapitalState::Withdrawable => {
                        v.settled_cents += l.cents;
                        v.withdrawable_cents += l.cents;
                    }
                    CapitalState::Pledged(_) | CapitalState::Reserved(_) => {
                        v.settled_cents += l.cents;
                        v.pledged_reserved_cents += l.cents;
                    }
                    CapitalState::PendingProceeds => v.pending_proceeds_cents += l.cents,
                    CapitalState::Delayed(_) => {
                        v.pending_proceeds_cents += l.cents;
                        v.delayed_blocked_cents += l.cents;
                    }
                    CapitalState::InTransit | CapitalState::Blocked(_) => {
                        v.delayed_blocked_cents += l.cents
                    }
                }
            }
            v
        })
        .collect();
        Snapshot {
            capital,
            lots: self.lots.clone(),
            aggregate_equity_cents: self.lots.iter().map(|l| l.cents).sum::<Money>()
                + self.unrealized
                - self.loan,
            loan_liability_cents: self.loan,
            hl_local_equity_cents: self.hl_local(),
            hl_required_cents: self.maintenance(),
            total_cost_cents: self.costs,
            action_executed: self.action,
        }
    }

    fn record(&mut self, code: &str, explanation: String) {
        self.ledger.push(LedgerEvent {
            sequence: self.ledger.len() + 1,
            tick: self.tick,
            code: code.into(),
            explanation,
            state: self.snapshot(),
        });
    }

    fn fail(&mut self, code: &str, explanation: String) {
        // A later recovery cannot erase a historical breach.
        if !self.failures.iter().any(|f| f.code == code) {
            self.failures.push(Failure {
                tick: self.tick,
                code: code.into(),
                explanation: explanation.clone(),
            });
            self.record(code, explanation);
        }
    }

    fn transfer(&mut self, source: &str, amount: Money, fee: Money, broken: bool) -> Result<()> {
        let i = self.lot_index(source)?;
        ensure!(
            amount > 0
                && self.lots[i].state == CapitalState::Withdrawable
                && self.lots[i].cents >= amount + fee,
            "UNAVAILABLE_CAPITAL: {source} cannot supply {} plus fee {} in state {:?}",
            dollars(amount),
            dollars(fee),
            self.lots[i].state
        );
        let id = format!("{source}-transfer-{}", self.tick);
        ensure!(
            !self.lots.iter().any(|l| l.id == id),
            "duplicate transfer id"
        );
        self.lots[i].cents -= amount + fee;
        self.costs += fee;
        self.lots.push(Lot {
            id: id.clone(),
            venue: self.lots[i].venue,
            cents: amount,
            state: CapitalState::Reserved(id.clone()),
        });
        self.record(
            "TRANSFER_RESERVED",
            format!(
                "{source}: reserve {}, charge {}; source funds no longer withdrawable",
                dollars(amount),
                dollars(fee)
            ),
        );
        let j = self.lots.len() - 1;
        self.lots[j].venue = Venue::InTransit;
        if broken {
            self.lots[j].state =
                CapitalState::Blocked("injected dispatch failure; recovery required".into());
            self.record("TRANSFER_BLOCKED", format!("{id}: dispatch failed after reservation; funds remain blocked and fee is not refunded"));
            anyhow::bail!("transfer failed after reservation; no atomic rollback");
        }
        self.lots[j].state = CapitalState::InTransit;
        let arrives = self.tick + self.f.rules.transfer_ticks;
        self.transfers.push(Transfer {
            lot: id.clone(),
            arrives,
        });
        self.record(
            "TRANSFER_SENT",
            format!(
                "{id}: Hyperliquid receives funds only at t={arrives}; no credit before arrival"
            ),
        );
        Ok(())
    }

    fn prepare(&mut self) -> Result<()> {
        match self.plan {
            Plan::Wallet => self.transfer(
                "wallet-ready",
                self.f.rules.wallet_transfer_cents,
                self.f.rules.wallet_fee_cents,
                self.break_step,
            )?,
            Plan::Withdraw => self.transfer(
                "poly-ready",
                self.f.rules.withdraw_transfer_cents,
                self.f.rules.withdraw_fee_cents,
                self.break_step,
            )?,
            Plan::Reduce => {
                let i = self.lot_index("hl-margin")?;
                ensure!(
                    self.lots[i].cents >= self.f.rules.reduce_fee_cents,
                    "cannot fund reduction fee"
                );
                self.lots[i].cents -= self.f.rules.reduce_fee_cents;
                self.costs += self.f.rules.reduce_fee_cents;
                self.record("REDUCTION_SUBMITTED", format!("Synthetic close request; charge {}. No margin relief until fill confirmation", dollars(self.f.rules.reduce_fee_cents)));
                self.constraints()?;
                ensure!(
                    self.failures.is_empty(),
                    "margin breached before reduction fill; no retroactive margin relief"
                );
                ensure!(
                    !self.break_step,
                    "synthetic reduction did not fill; fee remains spent, exposure unchanged"
                );
                self.reduced = true;
                self.record("POSITION_REDUCED", format!("Synthetic fill confirmed: existing exposure remaining {} bps; initial and maintenance requirements reduced by {}. Proposed action unchanged; existing strategy exposure is lower", self.f.rules.reduce_remaining_exposure_bps, dollars(self.relief())));
                self.transfer(
                    "wallet-ready",
                    self.f.rules.wallet_transfer_cents,
                    self.f.rules.wallet_fee_cents,
                    false,
                )?;
            }
            Plan::Finance => {
                let q = self.f.quote.clone();
                for id in &q.collateral_lots {
                    self.pledge(id, &q.id)?;
                }
                let collateral = self.quote_collateral();
                ensure!(
                    q.principal_cents <= collateral * q.max_ltv_bps / 10_000,
                    "synthetic quote exceeds eligible collateral capacity"
                );
                self.finance_ready = true;
                self.record("QUOTE_ACCEPTED", format!("{}: SYNTHETIC ONLY; principal {}, fee {}, horizon interest {} bps, maturity t={}; collateral remains pledged if funding fails", q.id, dollars(q.principal_cents), dollars(q.upfront_fee_cents), q.interest_bps_for_horizon, q.matures_at));
            }
        }
        Ok(())
    }

    fn quote_collateral(&self) -> Money {
        self.lots
            .iter()
            .filter(|l| l.state == CapitalState::Pledged(self.f.quote.id.clone()))
            .map(|l| l.cents)
            .sum()
    }

    fn settle_poly(&mut self) -> Result<()> {
        let i = self.lot_index("poly-payout")?;
        self.lots[i].state = CapitalState::Withdrawable;
        self.record("POLY_SETTLED", "Synthetic payout confirmed; pending claim becomes settled and withdrawable, not retroactively available".into());
        let fee = self.f.rules.payout_sweep_fee_cents;
        let amount = self.lots[i].cents - fee;
        self.transfer("poly-payout", amount, fee, false)
    }

    fn advance(&mut self) -> Result<()> {
        let r = self.f.rules.clone();
        // Deterministic ordering: arrivals, lifecycle, financing, action, price move,
        // horizon interest, constraints. Equal-tick arrivals are usable that tick.
        let arrived: Vec<_> = self
            .transfers
            .iter()
            .filter(|t| t.arrives == self.tick)
            .map(|t| t.lot.clone())
            .collect();
        for id in arrived {
            let i = self.lot_index(&id)?;
            self.lots[i].venue = Venue::Hyperliquid;
            self.lots[i].state = CapitalState::SettledLocked;
            self.record(
                "TRANSFER_ARRIVED",
                format!(
                    "{id}: {} now counts toward local Hyperliquid margin",
                    dollars(self.lots[i].cents)
                ),
            );
        }
        if self.tick == r.kalshi_close_at {
            self.record("KALSHI_CLOSED", "Market closed; determination and settlement are still separate future events. Contribution to Hyperliquid funding: $0.00".into());
        }
        if self.tick == r.kalshi_determination_at {
            self.record("KALSHI_DETERMINED", format!("Outcome determined; proceeds still pending until t={}. Commercial-access requirement remains UNMET", r.kalshi_settlement_at));
        }
        if self.tick == r.kalshi_settlement_at {
            let i = self.lot_index("kalshi-payout")?;
            self.lots[i].state = CapitalState::Withdrawable;
            self.record("KALSHI_SETTLED", "Synthetic proceeds now settled on Kalshi. No transfer adapter or commercial access; no automatic Hyperliquid credit".into());
        }
        if self.scenario == Scenario::Delayed && self.tick == r.poly_challenge_at {
            let i = self.lot_index("poly-payout")?;
            self.lots[i].state = CapitalState::Delayed("synthetic resolution challenge".into());
            self.record("POLY_CHALLENGE_DELAY", format!("{} pending proceeds delayed from t={} to t={}; funding contribution before settlement = $0.00", dollars(self.lots[i].cents), r.poly_settlement_at, r.poly_delayed_settlement_at));
        }
        let settle_at = if self.scenario == Scenario::Delayed {
            r.poly_delayed_settlement_at
        } else {
            r.poly_settlement_at
        };
        if self.tick == settle_at {
            self.settle_poly()?;
        }
        if self.plan == Plan::Finance && self.finance_ready && self.tick == self.f.quote.fund_at {
            if self.tick > self.f.quote.expires_at {
                self.fail("QUOTE_EXPIRED", "Quote expired before disbursement; no borrowing capacity is invented and collateral remains pledged pending recovery".into());
            } else if self.break_step {
                self.fail("FINANCING_NOT_DELIVERED", "Synthetic provider failed after collateral pledge. Loan principal not disbursed; liability remains zero; collateral stays pledged".into());
            } else {
                self.loan = self.f.quote.principal_cents;
                self.costs += self.f.quote.upfront_fee_cents;
                self.lots.push(Lot {
                    id: "financed-margin".into(),
                    venue: Venue::Hyperliquid,
                    cents: self.loan - self.f.quote.upfront_fee_cents,
                    state: CapitalState::SettledLocked,
                });
                self.record("SYNTHETIC_CREDIT_DELIVERED", format!("Provider delivers {} net to Hyperliquid and books {} liability; borrowed funds are not new equity", dollars(self.loan - self.f.quote.upfront_fee_cents), dollars(self.loan)));
            }
        }
        if self.tick == self.f.action.at {
            let required = self.f.action.initial_margin_cents - self.relief();
            if !self.failures.is_empty() {
                self.fail("ACTION_NOT_EXECUTED", "Earlier funding step failed; dependent action cancelled. Completed steps remain recorded".into());
            } else if self.hl_local() < required {
                self.fail(
                    "ACTION_NOT_FUNDED",
                    format!(
                        "Proposed action needs local equity {}; only {} has arrived",
                        dollars(required),
                        dollars(self.hl_local())
                    ),
                );
            } else {
                self.action = true;
                self.record("ACTION_EXECUTED", format!("{}; local equity {} meets initial requirement {}. Subsequent maintenance is checked separately", self.f.action.description, dollars(self.hl_local()), dollars(required)));
            }
        }
        if self.tick == self.f.price_move.at {
            let remaining = if self.reduced {
                r.reduce_remaining_exposure_bps
            } else {
                10_000
            };
            // Round loss upward to cents, never overstate collateral through truncation.
            let existing_loss =
                (self.f.price_move.existing_position_loss_cents * remaining + 9_999) / 10_000;
            self.unrealized = -existing_loss
                - if self.action {
                    self.f.price_move.proposed_position_loss_cents
                } else {
                    0
                };
            self.record("PRICE_MOVE", format!("Synthetic mark move: existing-position loss {}, proposed-position loss {}; no invented cash settlement of unrealized P&L", dollars(existing_loss), dollars(if self.action { self.f.price_move.proposed_position_loss_cents } else { 0 })));
        }
        if self.tick == self.f.horizon && self.loan > 0 {
            let interest = (self.f.quote.principal_cents * self.f.quote.interest_bps_for_horizon
                + 9_999)
                / 10_000;
            self.loan += interest;
            self.costs += interest;
            self.record("INTEREST_ACCRUED", format!("{} horizon interest accrued to liability. {} remains owed at maturity t={}, beyond this demo; repayment is not simulated", dollars(interest), dollars(self.loan), self.f.quote.matures_at));
        }
        Ok(())
    }

    fn constraints(&mut self) -> Result<()> {
        ensure!(
            self.lots.iter().all(|l| l.cents >= 0),
            "negative capital lot"
        );
        let s = self.snapshot();
        ensure!(
            s.aggregate_equity_cents == self.opening_equity - self.costs + self.unrealized,
            "ACCOUNTING_INVARIANT: equity was created, destroyed, or double-counted"
        );
        if s.hl_local_equity_cents < s.hl_required_cents {
            self.fail("HL_MARGIN_UNMET", format!("Aggregate marked equity {} {}; Hyperliquid has {} against local maintenance {}; shortfall {}. External pending/pledged capital is not margin in time", dollars(s.aggregate_equity_cents), if s.aggregate_equity_cents > 0 { "is positive" } else { "is not positive" }, dollars(s.hl_local_equity_cents), dollars(s.hl_required_cents), dollars(s.hl_required_cents - s.hl_local_equity_cents)));
        }
        if self.loan > self.quote_collateral() * self.f.quote.max_ltv_bps / 10_000 {
            self.fail("SYNTHETIC_LENDER_LIMIT", "Loan plus accrued interest exceeds the synthetic collateral limit; no real credit guarantee".into());
        }
        Ok(())
    }

    pub fn run(mut self) -> Result<Report> {
        self.record("INITIAL_STATE", "SYNTHETIC USD CENTS; marked equity includes pending claims at fixture marks. It is not spendable cash. Kalshi commercial access: UNMET".into());
        self.constraints()?;
        if self.failures.is_empty()
            && let Err(e) = self.prepare()
        {
            self.fail("PLAN_STEP_FAILED", e.to_string());
        }
        self.constraints()?;
        for tick in 1..=self.f.horizon {
            self.tick = tick;
            if let Err(e) = self.advance() {
                self.fail("PLAN_STEP_FAILED", e.to_string());
            }
            self.constraints()?;
            self.record("CHECKPOINT", "Local margin, synthetic lender limit, and equity conservation checked. Historical breaches remain latched".into());
        }
        let feasible = self.failures.is_empty() && self.action;
        self.record(if feasible { "PLAN_SURVIVES_HORIZON" } else { "PLAN_FAILED" }, format!("Decision is limited to t=0..{} and the entered rules. No liquidation fills or post-horizon repayment modeled", self.f.horizon));
        Ok(Report {
            fixture_id: self.f.id.clone(),
            fixture: self.f.clone(),
            plan: self.plan,
            scenario: self.scenario,
            injected_step_failure: self.break_step,
            feasible,
            failures: self.failures.clone(),
            ledger: self.ledger.clone(),
            final_state: self.snapshot(),
        })
    }
}

pub fn replay(
    fixture: &Fixture,
    plan: Plan,
    scenario: Scenario,
    break_step: bool,
) -> Result<Report> {
    Engine::new(fixture.clone(), plan, scenario, break_step)?.run()
}
