//! Constructed edge cases exercise accounting invariants; never runtime/demo balances.
use capital_core::{capital::*, margin::*, *};

fn d(s: &str) -> Decimal {
    Decimal::parse(s).unwrap()
}
fn observation(venue: &str, id: &str) -> AccountObservation {
    let mut o = AccountObservation::new(
        id.into(),
        venue,
        EvmAddress::parse("0x1111111111111111111111111111111111111111").unwrap(),
    );
    o.read_status = ReadStatus::Complete;
    o.evidence.push(EvidenceRef {
        id: id.into(),
        endpoint: "unit-test".into(),
        received_at_ms: 1000,
        source_time_ms: Some(1000),
        block: None,
    });
    o
}
fn fact(o: &mut AccountObservation, field: &str, network: &str, id: &str, value: FactValue) {
    o.facts.push(Fact {
        field: field.into(),
        asset: if network.is_empty() {
            None
        } else {
            Some(AssetId {
                network: network.into(),
                id: id.into(),
            })
        },
        value,
        evidence_id: o.evidence[0].id.clone(),
    });
}
fn num(o: &mut AccountObservation, name: &str, network: &str, id: &str, value: &str) {
    fact(o, name, network, id, FactValue::Amount(d(value)));
}
fn txt(o: &mut AccountObservation, name: &str, network: &str, id: &str, value: &str) {
    fact(o, name, network, id, FactValue::Text(value.into()));
}
fn policy() -> ReconcilePolicy {
    ReconcilePolicy {
        now_ms: 1000,
        max_age_ms: 100,
        max_skew_ms: 50,
        reservations: vec![],
    }
}
fn wallet() -> AccountObservation {
    let mut o = observation("evm", "wallet");
    txt(&mut o, "wallet.block_policy", "", "", "finalized");
    num(
        &mut o,
        "wallet.token_balance.USDC",
        "evm:42161",
        "usdc-contract",
        "10000.000001",
    );
    o
}
fn hl(equity: &str, withdrawable: &str) -> AccountObservation {
    let mut o = observation("hyperliquid", "hl");
    o.account_mode = Some("disabled".into());
    txt(&mut o, "account.role", "", "", "user");
    for (name, value) in [
        ("accountValue", equity),
        ("withdrawable", withdrawable),
        ("crossMaintenanceMarginUsed", "5"),
        ("totalNtlPos", "100"),
    ] {
        num(
            &mut o,
            &format!("default_dex.reported.{name}"),
            "hyperliquid",
            "reported-USD",
            value,
        );
    }
    num(&mut o, "default_dex.open_order_count", "", "", "0");
    num(&mut o, "position.szi", "hyperliquid-perp", "BTC", "1");
    txt(
        &mut o,
        "position.margin_mode",
        "hyperliquid-perp",
        "BTC",
        "cross",
    );
    num(&mut o, "position.leverage", "hyperliquid-perp", "BTC", "5");
    num(
        &mut o,
        "position.unrealizedPnl",
        "hyperliquid",
        "reported-USD:BTC",
        "-1",
    );
    num(
        &mut o,
        "position.positionValue",
        "hyperliquid",
        "reported-USD:BTC",
        "100",
    );
    o.margin_schedules.push(MarginSchedule {
        coin: "BTC".into(),
        tiers: vec![MarginTier {
            lower_bound: d("0"),
            max_leverage: 10,
        }],
        evidence_id: "hl".into(),
    });
    o
}
#[test]
fn local_margin_fails_even_with_large_settled_wallet_balance() {
    let out = reconcile(&[hl("4", "0"), wallet()], &policy());
    assert!(out.issues.iter().any(|i| i.code == "HL_MARGIN_UNMET"));
    assert_eq!(
        out.claims[0]
            .margin
            .as_ref()
            .unwrap()
            .maintenance_buffer
            .to_string(),
        "-1.000000"
    );
    assert!(out.claims[1].states.settled.amount.is_some());
    assert_eq!(out.funding_verdict, "UNDETERMINED");
}
#[test]
fn initial_maintenance_and_transfer_floor_have_distinct_meanings() {
    let out = reconcile(&[hl("50", "30")], &policy());
    assert!(out.issues.is_empty(), "{:?}", out.issues);
    let c = &out.claims[0];
    let m = c.margin.as_ref().unwrap();
    assert_eq!(m.initial_required.to_string(), "20.000000");
    assert_eq!(m.maintenance_required.to_string(), "5.000000");
    assert_eq!(c.states.settled.amount.as_ref().unwrap().to_string(), "51");
    assert_eq!(
        c.states.unrealized.amount.as_ref().unwrap().to_string(),
        "-1"
    );
}
#[test]
fn tier_integral_and_conservative_rounding_match_independent_cases() {
    let tiers = [
        MarginTier {
            lower_bound: d("0"),
            max_leverage: 20,
        },
        MarginTier {
            lower_bound: d("100"),
            max_leverage: 10,
        },
    ];
    for (n, expected) in [
        ("0", "0"),
        ("100", "2.500000"),
        ("200", "7.500000"),
        ("100.000001", "2.500001"),
    ] {
        assert_eq!(
            maintenance_for(&d(n), &tiers).unwrap().to_string(),
            expected
        );
    }
    assert!(
        maintenance_for(
            &d("10"),
            &[MarginTier {
                lower_bound: d("1"),
                max_leverage: 10
            }]
        )
        .is_err()
    );
}
#[test]
fn exact_precision_arithmetic_never_rounds_to_floats_or_wraps() {
    assert_eq!(
        d("10000000000000000.000001")
            .checked_sub(&d("0.000001"))
            .unwrap()
            .to_string(),
        "10000000000000000.000000"
    );
    assert!(
        Decimal::from_atoms(i128::MAX, 0)
            .unwrap()
            .checked_add(&d("1"))
            .is_err()
    );
    assert_eq!(
        d("0.000000001").divide_ceil(3, 6).unwrap().to_string(),
        "0.000001"
    );
}
#[test]
fn unified_mode_never_turns_per_dex_equity_into_an_extra_holding() {
    let mut o = hl("50", "30");
    o.account_mode = Some("unifiedAccount".into());
    num(&mut o, "spot.reported.total", "hyperliquid-spot", "0", "50");
    num(&mut o, "spot.reported.hold", "hyperliquid-spot", "0", "5");
    let out = reconcile(&[o], &policy());
    assert_eq!(out.claims.len(), 1);
    assert_eq!(out.claims[0].kind, "hl_spot");
    assert!(out.claims[0].states.withdrawable.amount.is_none());
    assert!(out.issues.iter().any(|i| i.code == "HL_MODE_UNSUPPORTED"));
}
#[test]
fn unknown_mode_and_isolated_positions_never_fall_back_to_cross() {
    for mode in ["default", "portfolioMargin", "newMode"] {
        let mut o = hl("50", "30");
        o.account_mode = Some(mode.into());
        let out = reconcile(&[o], &policy());
        assert!(
            out.claims
                .iter()
                .all(|c| c.margin.is_none() && c.after_local_reserves.amount.is_none())
        );
    }
    let mut o = hl("50", "30");
    o.facts
        .iter_mut()
        .find(|f| f.field == "position.margin_mode")
        .unwrap()
        .value = FactValue::Text("isolated".into());
    let out = reconcile(&[o], &policy());
    assert!(out.issues.iter().any(|i| i.code == "RECONCILIATION_GAP"));
    assert!(out.claims.iter().all(|c| c.margin.is_none()));
}
#[test]
fn partial_stale_duplicate_or_skewed_observations_never_supply_capacity() {
    for fault in ["partial", "stale", "duplicate", "skew"] {
        let mut o = wallet();
        let mut p = policy();
        match fault {
            "partial" => o.read_status = ReadStatus::Partial,
            "stale" => p.now_ms = 2000,
            "duplicate" => o.facts.push(o.facts[1].clone()),
            _ => {
                let mut e = o.evidence[0].clone();
                e.id = "late".into();
                e.source_time_ms = Some(900);
                o.evidence.push(e);
            }
        }
        let out = reconcile(&[o], &p);
        assert!(!out.issues.is_empty());
        if fault == "duplicate" {
            assert!(out.claims.is_empty());
            assert!(out.issues.iter().any(|i| i.code == "DUPLICATE_CLAIM"));
            continue;
        }
        assert!(out.claims[0].states.settled.amount.is_none(), "{fault}");
        assert!(out.claims[0].observed_quantity.amount.is_some());
    }
}
#[test]
fn identical_account_repeated_is_not_counted_twice() {
    let o = wallet();
    let out = reconcile(&[o.clone(), o], &policy());
    assert_eq!(out.claims.len(), 1);
    assert!(out.issues.iter().any(|i| i.code == "DUPLICATE_ACCOUNT"));
    assert!(out.claims[0].states.settled.amount.is_none());
}
#[test]
fn local_reservations_cannot_double_pledge_a_claim() {
    let o = hl("50", "30");
    let id = reconcile(std::slice::from_ref(&o), &policy()).claims[0]
        .id
        .clone();
    let mut p = policy();
    p.reservations = vec![
        Reservation {
            id: "a".into(),
            claim_id: id.clone(),
            amount: d("20"),
        },
        Reservation {
            id: "b".into(),
            claim_id: id,
            amount: d("20"),
        },
    ];
    let out = reconcile(std::slice::from_ref(&o), &p);
    assert!(
        out.issues
            .iter()
            .any(|i| i.code == "DOUBLE_PLEDGE_OR_OVERRESERVE")
    );
    assert!(out.claims[0].after_local_reserves.amount.is_none());
    p.reservations[1].amount = d("5");
    let out = reconcile(&[o], &p);
    assert!(out.issues.is_empty());
    assert_eq!(
        out.claims[0]
            .after_local_reserves
            .amount
            .as_ref()
            .unwrap()
            .to_string(),
        "5"
    );
    assert_eq!(
        out.claims[0]
            .states
            .withdrawable
            .amount
            .as_ref()
            .unwrap()
            .to_string(),
        "30"
    );
}
#[test]
fn prediction_resolution_hints_and_delay_never_become_cash() {
    for state in ["proposed", "disputed", "resolved"] {
        let mut o = observation("polymarket", "poly");
        num(
            &mut o,
            "position.size",
            "polymarket-predictions",
            "condition:123",
            "20",
        );
        fact(
            &mut o,
            "position.redeemable_hint",
            "polymarket-predictions",
            "condition:123",
            FactValue::Flag(true),
        );
        txt(
            &mut o,
            "market.lifecycle",
            "polymarket-condition",
            "condition",
            state,
        );
        let out = reconcile(&[o], &policy());
        let c = &out.claims[0];
        assert!(
            c.states.settled.amount.is_none()
                && c.states.withdrawable.amount.is_none()
                && c.states.pending_proceeds.amount.is_none()
        );
        assert!(c.after_local_reserves.amount.is_none());
        assert_eq!(
            c.states
                .delayed_blocked
                .amount
                .as_ref()
                .unwrap()
                .to_string(),
            "20"
        );
    }
}
#[test]
fn transaction_event_plus_snapshot_is_not_two_deposits_and_duplicates_are_idempotent() {
    let mut o = wallet();
    let event = ObservedActivity {
        event_id: "tx:1".into(),
        at_ms: 900,
        kind: "deposit".into(),
        evidence_id: "wallet".into(),
    };
    o.activities = vec![event.clone(), event];
    let out = reconcile(&[o], &policy());
    assert_eq!(out.claims.len(), 1);
    assert_eq!(
        out.claims[0]
            .states
            .settled
            .amount
            .as_ref()
            .unwrap()
            .to_string(),
        "10000.000001"
    );
    assert_eq!(
        out.ledger
            .iter()
            .filter(|e| e.kind == "venue_activity")
            .count(),
        1
    );
    assert_eq!(
        out.ledger
            .iter()
            .filter(|e| e.kind == "duplicate_event")
            .count(),
        1
    );
}
#[test]
fn newer_event_requires_snapshot_refresh() {
    let mut o = hl("50", "30");
    o.activities.push(ObservedActivity {
        event_id: "tx:1".into(),
        at_ms: 1001,
        kind: "withdraw".into(),
        evidence_id: "hl".into(),
    });
    let out = reconcile(&[o], &policy());
    assert!(out.issues.iter().any(|i| i.code == "EVENT_AFTER_SNAPSHOT"));
    assert!(out.claims[0].after_local_reserves.amount.is_none());
}

#[test]
fn spot_holds_reduce_local_reservation_ceiling() {
    let mut o = hl("50", "30");
    num(
        &mut o,
        "spot.reported.total",
        "hyperliquid-spot",
        "0",
        "100",
    );
    num(&mut o, "spot.reported.hold", "hyperliquid-spot", "0", "90");
    let id = reconcile(std::slice::from_ref(&o), &policy())
        .claims
        .iter()
        .find(|c| c.kind == "hl_spot")
        .unwrap()
        .id
        .clone();
    let mut p = policy();
    p.reservations.push(Reservation {
        id: "r".into(),
        claim_id: id,
        amount: d("20"),
    });
    let out = reconcile(&[o], &p);
    assert!(
        out.issues
            .iter()
            .any(|i| i.code == "DOUBLE_PLEDGE_OR_OVERRESERVE")
    );
}
#[test]
fn identical_symbols_on_different_networks_stay_distinct() {
    let a = wallet();
    let mut b = wallet();
    b.alias = "other-chain".into();
    b.evidence[0].id = "other-chain".into();
    for f in &mut b.facts {
        f.evidence_id = "other-chain".into();
        if let Some(asset) = &mut f.asset {
            asset.network = "evm:137".into();
        }
    }
    let out = reconcile(&[a, b], &policy());
    assert!(out.issues.is_empty());
    assert_eq!(out.claims.len(), 2);
    assert_ne!(out.claims[0].id, out.claims[1].id);
}
#[test]
fn conflicting_events_and_duplicate_reservation_ids_invalidate_capacity() {
    let mut o = hl("50", "30");
    let event = ObservedActivity {
        event_id: "tx".into(),
        at_ms: 900,
        kind: "deposit".into(),
        evidence_id: "hl".into(),
    };
    let mut changed = event.clone();
    changed.kind = "withdraw".into();
    o.activities = vec![event, changed];
    let out = reconcile(std::slice::from_ref(&o), &policy());
    assert!(out.issues.iter().any(|i| i.code == "CONFLICTING_EVENT"));
    assert!(out.claims[0].after_local_reserves.amount.is_none());
    o.activities.clear();
    let id = reconcile(std::slice::from_ref(&o), &policy()).claims[0]
        .id
        .clone();
    let mut p = policy();
    p.reservations = vec![
        Reservation {
            id: "same".into(),
            claim_id: id.clone(),
            amount: d("1"),
        },
        Reservation {
            id: "same".into(),
            claim_id: id,
            amount: d("1"),
        },
    ];
    let out = reconcile(&[o], &p);
    assert!(out.issues.iter().any(|i| i.code == "INVALID_RESERVATION"));
    assert!(out.claims[0].after_local_reserves.amount.is_none());
}
