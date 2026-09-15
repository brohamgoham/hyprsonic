use hyprsonic::{
    engine::{Engine, replay},
    model::*,
};

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../fixtures/lane1.json")).unwrap()
}

#[test]
fn delayed_payout_leaves_local_margin_unmet_despite_positive_aggregate_equity() {
    let r = replay(&fixture(), Plan::Wallet, Scenario::Delayed, false).unwrap();
    assert!(!r.feasible);
    assert!(r.final_state.action_executed);
    let e = r
        .ledger
        .iter()
        .find(|e| e.code == "HL_MARGIN_UNMET")
        .unwrap();
    assert_eq!(e.tick, 5);
    assert_eq!(e.state.hl_local_equity_cents, 250000);
    assert_eq!(e.state.hl_required_cents, 400000);
    assert_eq!(e.state.aggregate_equity_cents, 1409900);
    assert!(e.state.aggregate_equity_cents > e.state.hl_required_cents);
}

#[test]
fn settlement_delay_alone_changes_funding_outcome() {
    let f = fixture();
    let good = replay(&f, Plan::Wallet, Scenario::OnTime, false).unwrap();
    let delayed = replay(&f, Plan::Wallet, Scenario::Delayed, false).unwrap();
    assert!(good.feasible);
    assert!(!delayed.feasible);
    assert_eq!(
        good.final_state.capital[1].unrealized_cents,
        delayed.final_state.capital[1].unrealized_cents
    );
    assert!(!delayed.ledger.iter().any(|e| e.code == "POLY_SETTLED"));
    let poly = &delayed.final_state.capital[2];
    assert_eq!(poly.pending_proceeds_cents, 300000);
    assert_eq!(poly.delayed_blocked_cents, 300000);
    assert_eq!(poly.withdrawable_cents, 250000); // independent, already-settled lot only
}

#[test]
fn double_pledge_is_rejected() {
    let mut engine = Engine::new(fixture(), Plan::Finance, Scenario::Delayed, false).unwrap();
    engine.pledge("wallet-ready", "first").unwrap();
    assert!(
        engine
            .pledge("wallet-ready", "second")
            .unwrap_err()
            .to_string()
            .contains("DOUBLE_PLEDGE")
    );
    assert!(engine.pledge("wallet-pledged", "second").is_err());
    assert!(engine.pledge("poly-payout", "second").is_err());
}

#[test]
fn pledged_funds_cannot_also_be_withdrawn() {
    let mut engine = Engine::new(fixture(), Plan::Wallet, Scenario::Delayed, false).unwrap();
    engine.pledge("wallet-ready", "other-plan-step").unwrap();
    let r = engine.run().unwrap();
    assert!(!r.feasible);
    assert!(!r.final_state.action_executed);
    assert!(
        r.failures
            .iter()
            .any(|f| f.explanation.contains("UNAVAILABLE_CAPITAL"))
    );
}

#[test]
fn full_plan_matrix_and_cost_deltas() {
    let f = fixture();
    for plan in Plan::ALL {
        assert!(replay(&f, plan, Scenario::OnTime, false).unwrap().feasible);
    }
    let expected = [
        (Plan::Wallet, false, 100),
        (Plan::Withdraw, false, 200),
        (Plan::Reduce, true, 3100),
        (Plan::Finance, true, 3600),
    ];
    for (plan, feasible, cost) in expected {
        let r = replay(&f, plan, Scenario::Delayed, false).unwrap();
        assert_eq!(r.feasible, feasible);
        assert_eq!(r.final_state.total_cost_cents, cost);
    }
    let baseline = replay(&f, Plan::Wallet, Scenario::OnTime, false).unwrap();
    let reduced = replay(&f, Plan::Reduce, Scenario::Delayed, false).unwrap();
    assert_eq!(
        reduced.final_state.total_cost_cents - baseline.final_state.total_cost_cents,
        2900
    );
}

#[test]
fn credit_adds_matching_liability_and_reserves_collateral() {
    let r = replay(&fixture(), Plan::Finance, Scenario::Delayed, false).unwrap();
    let e = r
        .ledger
        .iter()
        .find(|e| e.code == "SYNTHETIC_CREDIT_DELIVERED")
        .unwrap();
    assert_eq!(e.state.loan_liability_cents, 320000);
    assert_eq!(e.state.aggregate_equity_cents, 1708000); // 17100 opening equity less $20 fee
    assert_eq!(e.state.capital[0].withdrawable_cents, 0);
    assert_eq!(e.state.capital[2].withdrawable_cents, 0);
    assert_eq!(r.final_state.loan_liability_cents, 321600);
}

#[test]
fn broken_steps_do_not_roll_back_completed_reservations_or_costs() {
    let f = fixture();
    for plan in Plan::ALL {
        let r = replay(&f, plan, Scenario::Delayed, true).unwrap();
        assert!(!r.feasible);
        assert!(!r.final_state.action_executed);
        if plan == Plan::Finance {
            assert_eq!(r.final_state.loan_liability_cents, 0);
            assert_eq!(r.final_state.capital[2].pledged_reserved_cents, 250000);
        } else {
            assert!(r.final_state.total_cost_cents > 0);
        }
        if plan == Plan::Wallet || plan == Plan::Withdraw {
            assert!(
                r.final_state
                    .lots
                    .iter()
                    .any(|l| matches!(l.state, CapitalState::Blocked(_)))
            );
        }
    }
}

#[test]
fn transfer_arrival_is_not_assumed_atomic() {
    let mut f = fixture();
    f.rules.transfer_ticks = 3;
    let r = replay(&f, Plan::Wallet, Scenario::OnTime, false).unwrap();
    assert!(!r.feasible);
    assert!(r.failures.iter().any(|f| f.code == "ACTION_NOT_FUNDED"));
    assert!(
        r.ledger
            .iter()
            .any(|e| e.tick == 3 && e.code == "TRANSFER_ARRIVED")
    );
    assert!(!r.final_state.action_executed); // no backdated success
}

#[test]
fn expired_quote_creates_no_credit() {
    let mut f = fixture();
    f.quote.expires_at = 0;
    let r = replay(&f, Plan::Finance, Scenario::Delayed, false).unwrap();
    assert!(!r.feasible);
    assert_eq!(r.final_state.loan_liability_cents, 0);
    assert!(r.failures.iter().any(|f| f.code == "QUOTE_EXPIRED"));
}

#[test]
fn kalshi_determination_does_not_release_cash() {
    let r = replay(&fixture(), Plan::Wallet, Scenario::OnTime, false).unwrap();
    let close = r.ledger.iter().find(|e| e.code == "KALSHI_CLOSED").unwrap();
    let determination = r
        .ledger
        .iter()
        .find(|e| e.code == "KALSHI_DETERMINED")
        .unwrap();
    assert!(close.tick < determination.tick);
    assert_eq!(determination.state.capital[3].withdrawable_cents, 0);
    assert_eq!(
        determination.state.capital[3].pending_proceeds_cents,
        500000
    );
    assert!(!r.fixture.rules.kalshi_commercial_access_met);
}

#[test]
fn historical_margin_breach_stays_failed_after_late_cash_arrives() {
    let mut f = fixture();
    f.horizon = 9;
    f.rules.poly_delayed_settlement_at = 7;
    let r = replay(&f, Plan::Wallet, Scenario::Delayed, false).unwrap();
    assert!(r.final_state.hl_local_equity_cents > r.final_state.hl_required_cents);
    assert!(!r.feasible);
    assert_eq!(
        r.failures
            .iter()
            .find(|f| f.code == "HL_MARGIN_UNMET")
            .unwrap()
            .tick,
        5
    );
}

#[test]
fn invalid_or_live_inputs_fail_closed() {
    let mut f = fixture();
    f.synthetic = false;
    assert!(f.validate().is_err());
    f.synthetic = true;
    f.capital.hl_settled_cents = -1;
    assert!(f.validate().is_err());
    f.capital.hl_settled_cents = i64::MAX;
    assert!(f.validate().is_err());
    let mut json = serde_json::to_value(fixture()).unwrap();
    json["surprise_live_adapter"] = true.into();
    assert!(serde_json::from_value::<Fixture>(json).is_err());
}

#[test]
fn reduction_fee_cannot_breach_margin_before_the_fill_grants_relief() {
    let mut f = fixture();
    f.capital.hl_settled_cents = 351000;
    let r = replay(&f, Plan::Reduce, Scenario::OnTime, false).unwrap();
    assert!(!r.feasible);
    assert!(
        r.failures
            .iter()
            .any(|f| f.code == "HL_MARGIN_UNMET" && f.tick == 0)
    );
    assert!(!r.ledger.iter().any(|e| e.code == "POSITION_REDUCED"));
}

#[test]
fn exact_hl_boundary_and_one_cent_breach_are_distinct() {
    let mut f = fixture();
    f.capital.hl_settled_cents = 550000;
    assert!(
        replay(&f, Plan::Wallet, Scenario::Delayed, false)
            .unwrap()
            .feasible
    );
    f.capital.hl_settled_cents -= 1;
    let r = replay(&f, Plan::Wallet, Scenario::Delayed, false).unwrap();
    assert!(!r.feasible);
    assert_eq!(
        r.final_state.hl_required_cents - r.final_state.hl_local_equity_cents,
        1
    );
}

#[test]
fn ledger_is_deterministic_and_each_snapshot_conserves_equity() {
    let f = fixture();
    for plan in Plan::ALL {
        let a = replay(&f, plan, Scenario::Delayed, false).unwrap();
        let b = replay(&f, plan, Scenario::Delayed, false).unwrap();
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
        let opening = a.ledger[0].state.aggregate_equity_cents;
        for (i, e) in a.ledger.iter().enumerate() {
            assert_eq!(e.sequence, i + 1);
            let pnl: i64 = e.state.capital.iter().map(|v| v.unrealized_cents).sum();
            assert_eq!(
                e.state.aggregate_equity_cents,
                opening - e.state.total_cost_cents + pnl
            );
        }
    }
}

#[test]
fn cli_failure_exit_and_json_evidence_are_machine_readable() {
    let binary = env!("CARGO_BIN_EXE_hyprsonic");
    let result = std::process::Command::new(binary)
        .args([
            "replay",
            "--scenario",
            "delayed",
            "--plan",
            "wallet",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["feasible"], false);
    assert_eq!(report["fixture"]["synthetic"], true);
    assert_eq!(report["failures"][0]["code"], "HL_MARGIN_UNMET");
    let invalid = std::process::Command::new(binary)
        .args(["replay", "--plan", "wallet"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
}
