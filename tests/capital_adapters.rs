use capital_core::*;
use hyprsonic::observe::adapters::{
    parse_hl_activity, parse_hl_perps, parse_margin_schedules, parse_market_lifecycle,
};
use serde_json::json;
fn evidence() -> EvidenceRef {
    EvidenceRef {
        id: "1".into(),
        endpoint: "unit-test".into(),
        received_at_ms: 1000,
        source_time_ms: None,
        block: None,
    }
}
#[test]
fn metadata_uses_explicit_tiers_and_rejects_missing_or_duplicate_tables() {
    let mut v = json!({"universe":[{"name":"BTC","maxLeverage":20,"marginTableId":51}],"marginTables":[[51,{"marginTiers":[{"lowerBound":"0","maxLeverage":20},{"lowerBound":"100","maxLeverage":10}]}]]});
    let schedules = parse_margin_schedules(&v, &evidence()).unwrap();
    assert_eq!(schedules[0].tiers.len(), 2);
    assert_eq!(schedules[0].evidence_id, "1");
    v["universe"][0]["marginTableId"] = json!(99);
    assert!(parse_margin_schedules(&v, &evidence()).is_err());
    v["universe"][0]["marginTableId"] = json!(51);
    let duplicate = v["marginTables"][0].clone();
    v["marginTables"].as_array_mut().unwrap().push(duplicate);
    assert!(parse_margin_schedules(&v, &evidence()).is_err());
    let simple = json!({"universe":[{"name":"HYPE","maxLeverage":10}],"marginTables":[]});
    assert_eq!(
        parse_margin_schedules(&simple, &evidence()).unwrap()[0].tiers[0].max_leverage,
        10
    );
}
#[test]
fn lifecycle_requires_matching_complete_conditions_and_never_emits_cash() {
    let ids = vec![format!("0x{}", "1".repeat(64))];
    for (status, closed, expected) in [
        ("resolved", true, "resolved"),
        ("disputed", true, "disputed"),
        ("", true, "closed_resolution_unverified"),
        ("", false, "open_or_unresolved"),
    ] {
        let v = json!([{"conditionId":ids[0],"closed":closed,"umaResolutionStatus":status}]);
        let facts = parse_market_lifecycle(&v, &ids, &evidence()).unwrap();
        assert_eq!(facts.len(), 1);
        assert!(matches!(&facts[0].value, FactValue::Text(s) if s == expected));
    }
    assert!(parse_market_lifecycle(&json!([]), &ids, &evidence()).is_err());
    assert!(
        parse_market_lifecycle(
            &json!([{"conditionId":"wrong","closed":true}]),
            &ids,
            &evidence()
        )
        .is_err()
    );
    let row = json!({"conditionId":ids[0],"closed":true});
    assert!(parse_market_lifecycle(&json!([row.clone(), row]), &ids, &evidence()).is_err());
}
#[test]
fn ledger_identity_distinguishes_multiple_deltas_in_one_transaction() {
    let mut event = json!({"time":1000,"hash":format!("0x{}","a".repeat(64)),"delta":{"type":"deposit","usdc":"1"}});
    let a = parse_hl_activity(&event, &evidence()).unwrap();
    assert_eq!(
        a.event_id,
        parse_hl_activity(&event, &evidence()).unwrap().event_id
    );
    event["delta"]["usdc"] = json!("2");
    assert_ne!(
        a.event_id,
        parse_hl_activity(&event, &evidence()).unwrap().event_id
    );
    event["hash"] = json!("not-a-tx");
    assert!(parse_hl_activity(&event, &evidence()).is_err());
}
#[test]
fn risk_parsing_requires_position_mode_and_unique_coin() {
    let position = json!({"position":{"coin":"BTC","leverage":{"type":"cross","value":10},"szi":"1","positionValue":"100","unrealizedPnl":"1","marginUsed":"10"}});
    let mut v = json!({"time":1000,"marginSummary":{"accountValue":"20","totalRawUsd":"19","totalMarginUsed":"10","totalNtlPos":"100"},"withdrawable":"10","crossMaintenanceMarginUsed":"5","assetPositions":[position.clone()]});
    let facts = parse_hl_perps(&v, &evidence()).unwrap();
    assert!(facts.iter().any(|f| f.field == "position.margin_mode"));
    v["assetPositions"].as_array_mut().unwrap().push(position);
    assert!(parse_hl_perps(&v, &evidence()).is_err());
    v["assetPositions"].as_array_mut().unwrap().pop();
    v["assetPositions"][0]["position"]["leverage"]["type"] = json!("unrecognized");
    assert!(parse_hl_perps(&v, &evidence()).is_err());
}
#[test]
fn placeholder_config_fails_before_network_and_help_is_usable() {
    let bin = env!("CARGO_BIN_EXE_hyprsonic");
    let out = std::process::Command::new(bin)
        .args([
            "capital",
            "--config",
            "config/accounts.example.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    for command in ["capital", "explain"] {
        let out = std::process::Command::new(bin)
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(!out.stdout.is_empty());
    }
}
