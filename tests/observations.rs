use capital_core::*;
use hyprsonic::observe::{
    adapters::{Source, hex_integer, parse_poly},
    config::{Account, Config},
    io::{Context, LocalJournal, Reply, Request, Transport},
};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Mutex};
const ADDRESS: &str = "0x1111111111111111111111111111111111111111";
struct FixedClock;
impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        1000
    }
}
#[derive(Default)]
struct Memory(Mutex<Vec<Vec<u8>>>);
impl EvidenceStore for Memory {
    fn append(&self, record: &[u8]) -> Result<String, String> {
        let mut v = self.0.lock().unwrap();
        v.push(record.to_vec());
        Ok(v.len().to_string())
    }
}
struct Mock<F>(F);
impl<F: Fn(&Request) -> Result<Reply, String> + Sync> Transport for Mock<F> {
    fn send(&self, r: &Request) -> Result<Reply, String> {
        (self.0)(r)
    }
}
fn reply(v: Value) -> Result<Reply, String> {
    Ok(Reply {
        status: 200,
        body: serde_json::to_vec(&v).unwrap(),
    })
}
fn evidence() -> EvidenceRef {
    EvidenceRef {
        id: "1".into(),
        endpoint: "test".into(),
        received_at_ms: 1000,
        source_time_ms: None,
        block: None,
    }
}
fn position(n: usize) -> Value {
    json!({"proxyWallet":ADDRESS,"conditionId":format!("0x{:064x}",n+1),"asset":(n+1).to_string(),"size":"0.000001","currentValue":"0.0000005","cashPnl":"-0.1","redeemable":true})
}
fn poly() -> Account {
    Account::Polymarket {
        alias: "poly".into(),
        address: ADDRESS.into(),
    }
}

#[test]
fn polymarket_redeemable_is_a_hint_and_amounts_keep_source_precision() {
    let out = parse_poly(
        &[position(0)],
        &EvmAddress::parse(ADDRESS).unwrap(),
        &evidence(),
        &mut HashSet::new(),
    )
    .unwrap();
    assert!(out.iter().any(|f| f.field == "position.redeemable_hint"));
    assert!(
        !out.iter()
            .any(|f| f.field.contains("withdrawable") || f.field.contains("settled"))
    );
    assert!(matches!(&out[0].value,FactValue::Amount(a) if a.to_string()=="0.000001"));
    let mut row = position(1);
    row["size"] = serde_json::from_str("123456789123456789.123456").unwrap();
    let out = parse_poly(
        &[row],
        &EvmAddress::parse(ADDRESS).unwrap(),
        &evidence(),
        &mut HashSet::new(),
    )
    .unwrap();
    assert!(
        matches!(&out[0].value,FactValue::Amount(a) if a.to_string()=="123456789123456789.123456")
    );
}
#[test]
fn mismatched_holding_wallet_and_duplicate_positions_fail() {
    let mut row = position(0);
    row["proxyWallet"] = json!("0x2222222222222222222222222222222222222222");
    assert!(
        parse_poly(
            &[row],
            &EvmAddress::parse(ADDRESS).unwrap(),
            &evidence(),
            &mut HashSet::new()
        )
        .is_err()
    );
    assert!(
        parse_poly(
            &[position(0), position(0)],
            &EvmAddress::parse(ADDRESS).unwrap(),
            &evidence(),
            &mut HashSet::new()
        )
        .is_err()
    );
}
#[test]
fn empty_positions_is_an_observed_empty_page_not_zero_cash() {
    let mock = Mock(|r: &Request| {
        assert!(r.query.contains(&("sizeThreshold".into(), "0".into())));
        assert!(r.query.contains(&("includeArchived".into(), "true".into())));
        reply(json!([]))
    });
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let account = poly();
    let mut o = Source {
        account: &account,
        context: &context,
    }
    .observe()
    .unwrap();
    o.finish(1000, 100);
    assert!(matches!(o.read_status, ReadStatus::Complete));
    assert_eq!(o.facts.len(), 1);
    assert_eq!(o.facts[0].field, "positions.count");
    assert!(o.limitations.iter().any(|s| s.contains("zero cash")));
}
#[test]
fn pagination_fetches_the_next_page_and_detects_bound() {
    for full in [false, true] {
        let calls = Mutex::new(0);
        let mock = Mock(|r: &Request| {
            *calls.lock().unwrap() += 1;
            let offset = r
                .query
                .iter()
                .find(|(k, _)| k == "offset")
                .unwrap()
                .1
                .parse::<usize>()
                .unwrap();
            let count = if full || offset == 0 { 500 } else { 1 };
            reply(Value::Array(
                (offset..offset + count).map(position).collect(),
            ))
        });
        let memory = Memory::default();
        let context = Context {
            transport: &mock,
            store: &memory,
            clock: &FixedClock,
        };
        let account = poly();
        let mut o = Source {
            account: &account,
            context: &context,
        }
        .observe()
        .unwrap();
        o.finish(1000, 100);
        if full {
            assert_eq!(*calls.lock().unwrap(), 21);
            assert!(o.issues.iter().any(|i| i.code == "PAGINATION_LIMIT"));
        } else {
            assert_eq!(*calls.lock().unwrap(), 2);
            assert!(matches!(o.read_status, ReadStatus::Complete));
            assert_eq!(o.facts.len(), 501 * 4 + 1);
        }
    }
}
#[test]
fn a_failed_later_page_keeps_evidence_and_never_claims_a_complete_count() {
    let mock = Mock(|r: &Request| {
        if r.query.iter().any(|(k, v)| k == "offset" && v == "0") {
            reply(Value::Array((0..500).map(position).collect()))
        } else {
            Ok(Reply {
                status: 401,
                body: b"provider secret".to_vec(),
            })
        }
    });
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let account = poly();
    let mut o = Source {
        account: &account,
        context: &context,
    }
    .observe()
    .unwrap();
    o.finish(1000, 100);
    assert!(matches!(o.read_status, ReadStatus::Partial));
    assert_eq!(o.facts.len(), 2000);
    assert!(!o.facts.iter().any(|f| f.field == "positions.count"));
    assert!(
        !String::from_utf8(memory.0.lock().unwrap()[1].clone())
            .unwrap()
            .contains("provider secret")
    );
}
#[test]
fn timeout_is_unavailable_not_an_empty_success() {
    let mock = Mock(|_: &Request| Err("TIMEOUT".into()));
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let account = poly();
    let mut o = Source {
        account: &account,
        context: &context,
    }
    .observe()
    .unwrap();
    o.finish(1000, 100);
    assert!(matches!(o.read_status, ReadStatus::Unavailable));
    assert!(o.facts.is_empty());
    assert_eq!(memory.0.lock().unwrap().len(), 1);
}
#[test]
fn rpc_errors_and_urls_cannot_leak_provider_secrets() {
    let mock = Mock(|_: &Request| {
        reply(json!({"jsonrpc":"2.0","id":1,"error":{"message":"https://example.org/SECRET"}}))
    });
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let err = context
        .read(
            "a",
            "evm/read",
            Request {
                url: "https://example.org/SECRET".into(),
                body: Some(json!({"jsonrpc":"2.0","method":"eth_chainId"})),
                query: vec![],
            },
        )
        .err()
        .unwrap();
    assert!(!err.contains("SECRET"));
    let stored = String::from_utf8(memory.0.lock().unwrap()[0].clone()).unwrap();
    assert!(!stored.contains("SECRET"));
    assert!(stored.contains("RPC_ERROR"));
}
#[test]
fn agent_addresses_are_rejected_before_reading_balances() {
    let mock = Mock(|r: &Request| {
        assert_eq!(r.body.as_ref().unwrap()["type"], "userRole");
        reply(json!({"role":"agent"}))
    });
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let account = Account::Hyperliquid {
        alias: "hl".into(),
        address: ADDRESS.into(),
    };
    let o = Source {
        account: &account,
        context: &context,
    }
    .observe()
    .unwrap();
    assert!(
        o.issues
            .iter()
            .any(|i| i.code == "ACCOUNT_IDENTITY_UNSUPPORTED")
    );
    assert_eq!(o.evidence.len(), 1);
}
#[test]
fn unknown_mode_is_visible_and_never_assigns_a_collateral_pool() {
    let mock = Mock(
        |r: &Request| match r.body.as_ref().unwrap()["type"].as_str().unwrap() {
            "userRole" => reply(json!({"role":"user"})),
            "userAbstraction" => reply(json!("newMode")),
            "clearinghouseState" => reply(
                json!({"time":1000,"marginSummary":{"accountValue":"10","totalMarginUsed":"1","totalNtlPos":"5","totalRawUsd":"9"},"withdrawable":"2","crossMaintenanceMarginUsed":"1","assetPositions":[]}),
            ),
            "spotClearinghouseState" => reply(json!({"balances":[]})),
            "openOrders" => reply(json!([])),
            _ => panic!("unexpected request"),
        },
    );
    let memory = Memory::default();
    let context = Context {
        transport: &mock,
        store: &memory,
        clock: &FixedClock,
    };
    let account = Account::Hyperliquid {
        alias: "hl".into(),
        address: ADDRESS.into(),
    };
    let mut o = Source {
        account: &account,
        context: &context,
    }
    .observe()
    .unwrap();
    o.finish(1000, 100);
    assert!(matches!(o.read_status, ReadStatus::Partial));
    assert!(matches!(o.collateral_domain, CollateralDomain::Unresolved));
    assert_eq!(o.account_mode.as_deref(), Some("newMode"));
}
#[test]
fn checked_hex_does_not_truncate_uint256() {
    assert_eq!(hex_integer(&json!("0x0000000f")).unwrap(), 15);
    assert!(hex_integer(&json!(format!("0x{}", "f".repeat(64)))).is_err());
    assert!(hex_integer(&json!("0x")).is_err());
}
#[test]
fn configuration_rejects_duplicates_secrets_and_bad_precision() {
    let mut config = json!({"max_age_seconds":120,"accounts":[{"kind":"polymarket","alias":"p","address":ADDRESS}]});
    let parsed: Config = serde_json::from_value(config.clone()).unwrap();
    parsed.validate().unwrap();
    let duplicate = config["accounts"][0].clone();
    config["accounts"].as_array_mut().unwrap().push(duplicate);
    assert!(
        serde_json::from_value::<Config>(config)
            .unwrap()
            .validate()
            .is_err()
    );
    let invalid = json!({"max_age_seconds":120,"accounts":[{"kind":"polymarket","alias":"p","address":ADDRESS,"private_key":"DO_NOT_ACCEPT"}]});
    assert!(serde_json::from_value::<Config>(invalid).is_err());
    assert!(
        serde_json::from_str::<Config>(include_str!("../config/accounts.example.json"))
            .unwrap()
            .validate()
            .is_err()
    );
}
#[test]
fn journal_is_private_exclusive_and_rejects_symlink_paths() {
    let base = std::env::temp_dir().join(format!("hyprsonic-journal-test-{}", std::process::id()));
    let journal = LocalJournal::create(&base, 1).unwrap();
    journal.append(b"{}").unwrap();
    assert!(LocalJournal::create(&base, 1).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        assert_eq!(
            std::fs::metadata(journal.path.join("000001.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&journal.path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        let link = base.join("link");
        symlink(&journal.path, &link).unwrap();
        assert!(LocalJournal::create(&link, 2).is_err());
    }
    std::fs::remove_dir_all(base).unwrap();
}
#[test]
fn invalid_config_cli_never_makes_a_live_request() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hyprsonic"))
        .args([
            "observe",
            "--config",
            "config/accounts.example.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8(output.stderr).unwrap().contains("YOUR_"));
}

#[test]
fn sanitized_live_response_shapes_decode_without_promoting_claims_to_cash() {
    let hl: Value =
        serde_json::from_str(include_str!("captures/hyperliquid-account.sanitized.json")).unwrap();
    let facts = hyprsonic::observe::adapters::parse_hl_perps(&hl, &evidence()).unwrap();
    assert_eq!(facts.len(), 6);
    let poly: Vec<Value> =
        serde_json::from_str(include_str!("captures/polymarket-positions.sanitized.json")).unwrap();
    let facts = parse_poly(
        &poly,
        &EvmAddress::parse(ADDRESS).unwrap(),
        &evidence(),
        &mut HashSet::new(),
    )
    .unwrap();
    assert_eq!(facts.len(), 8);
    assert!(!facts.iter().any(|f| f.field.contains("withdrawable")));
}
#[test]
fn application_runs_through_account_and_clock_ports_and_marks_staleness() {
    struct Reader(&'static str);
    impl AccountSource for Reader {
        fn observe(&self) -> Result<AccountObservation, String> {
            let mut o =
                AccountObservation::new(self.0.into(), "test", EvmAddress::parse(ADDRESS).unwrap());
            let mut e = evidence();
            e.received_at_ms = 1;
            o.evidence.push(e);
            Ok(o)
        }
    }
    let a = Reader("a");
    let b = Reader("b");
    let result = hyprsonic::observe::collect(&[&a, &b], &FixedClock, 10).unwrap();
    assert_eq!(
        result.iter().map(|o| o.alias.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(
        result
            .iter()
            .all(|o| matches!(o.read_status, ReadStatus::Partial))
    );
    assert!(hyprsonic::observe::collect(&[], &FixedClock, 10).is_err());
}

#[test]
fn malformed_orders_are_not_reported_as_a_successful_order_count() {
    use hyprsonic::observe::adapters::parse_hl_orders;
    assert!(parse_hl_orders(&json!([123]), &evidence()).is_err());
    let order = json!({"oid":1,"coin":"BTC","side":"B","timestamp":1000,"sz":"1","limitPx":"1"});
    assert!(parse_hl_orders(&json!([order.clone(), order.clone()]), &evidence()).is_err());
    assert!(parse_hl_orders(&json!([order]), &evidence()).is_ok());
}
