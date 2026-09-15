use super::{
    config::{Account, Token},
    io::{Context, Request},
};
use capital_core::*;
use serde_json::{Value, json};
use std::collections::HashSet;

pub struct Source<'a> {
    pub account: &'a Account,
    pub context: &'a Context<'a>,
}
fn amount(v: &Value) -> Result<Decimal, String> {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err("expected decimal string/number".into()),
    };
    Decimal::parse(&s).map_err(str::to_owned)
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing text field {key}"))
}
fn list<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array field {key}"))
}
fn numeric(v: &Value, key: &str) -> Result<Decimal, String> {
    amount(
        v.get(key)
            .ok_or_else(|| format!("missing numeric field {key}"))?,
    )
}
fn fact(field: &str, asset: Option<AssetId>, value: FactValue, e: &EvidenceRef) -> Fact {
    Fact {
        field: field.into(),
        asset,
        value,
        evidence_id: e.id.clone(),
    }
}
fn reported_asset(network: &str, id: &str) -> Option<AssetId> {
    Some(AssetId {
        network: network.into(),
        id: id.into(),
    })
}
fn parse_into(o: &mut AccountObservation, scope: &str, result: Result<Vec<Fact>, String>) {
    match result { Ok(f)=>o.facts.extend(f),Err(_)=>o.issue("SCHEMA_ERROR",scope,"Response has missing, conflicting or unsupported fields/precision; inspect private evidence.") }
}
impl Source<'_> {
    fn read(
        &self,
        o: &mut AccountObservation,
        endpoint: &str,
        request: Request,
    ) -> Result<Option<(Value, EvidenceRef)>, String> {
        match self.context.read(&o.alias, endpoint, request) {
            Ok((v, e)) => {
                o.evidence.push(e.clone());
                Ok(Some((v, e)))
            }
            Err(e) if e.starts_with("JOURNAL_WRITE_FAILED") => Err(e),
            Err(e) => {
                o.issue("READ_FAILED", endpoint, &e);
                Ok(None)
            }
        }
    }
    fn hl_read(
        &self,
        o: &mut AccountObservation,
        kind: &str,
    ) -> Result<Option<(Value, EvidenceRef)>, String> {
        self.read(
            o,
            &format!("hyperliquid/info/{kind}"),
            Request {
                url: "https://api.hyperliquid.xyz/info".into(),
                body: Some(json!({"type":kind,"user":o.account.address.as_str()})),
                query: vec![],
            },
        )
    }
    fn hl(&self, o: &mut AccountObservation) -> Result<(), String> {
        o.limitations.extend([
            "Only the configured account is observed; no subaccount, vault-deposit or HIP-3 enumeration.".into(),
            "Default perp DEX and spot responses are separate raw views; NEVER sum their equity.".into(),
            "No source timestamp in an endpoint means upstream freshness is unknown, even after a fresh fetch.".into(),
        ]);
        if let Some((v, e)) = self.hl_read(o, "userRole")? {
            match text(&v, "role") {
                Ok(role) => {
                    o.facts
                        .push(fact("account.role", None, FactValue::Text(role.into()), &e));
                    if !matches!(role, "user" | "subAccount" | "vault") {
                        o.issue("ACCOUNT_IDENTITY_UNSUPPORTED","userRole","Use the actual user/subaccount holding address, not an agent or missing account.");
                        return Ok(());
                    }
                    if role == "vault" {
                        o.limitations.push("Vault account observed; depositor withdrawal eligibility is not modeled.".into());
                    }
                }
                Err(_) => {
                    o.issue("SCHEMA_ERROR", "userRole", "Missing account role.");
                    return Ok(());
                }
            }
        } else {
            return Ok(());
        }
        if let Some((v, _)) = self.hl_read(o, "userAbstraction")? {
            if let Some(mode) = v.as_str() {
                o.account_mode = Some(mode.into());
                if !matches!(
                    mode,
                    "unifiedAccount"
                        | "portfolioMargin"
                        | "disabled"
                        | "default"
                        | "dexAbstraction"
                ) {
                    o.issue(
                        "UNSUPPORTED_MODE",
                        "userAbstraction",
                        "Unknown account mode; no fallback to standard rules.",
                    );
                }
            } else {
                o.issue(
                    "SCHEMA_ERROR",
                    "userAbstraction",
                    "Expected explicit account mode.",
                );
            }
        }
        if let Some((v, e)) = self.hl_read(o, "clearinghouseState")? {
            parse_into(o, "clearinghouseState", parse_hl_perps(&v, &e));
        }
        if let Some((v, e)) = self.hl_read(o, "spotClearinghouseState")? {
            parse_into(o, "spotClearinghouseState", parse_hl_spot(&v, &e));
        }
        if let Some((v, e)) = self.hl_read(o, "openOrders")? {
            parse_into(o, "openOrders", parse_hl_orders(&v, &e));
            o.limitations.push("Open-order payload captured; exact reserved collateral is not inferred from order count.".into());
        }
        Ok(())
    }
    fn poly(&self, o: &mut AccountObservation) -> Result<(), String> {
        o.limitations.extend([
            "Positions API v1 observation only: no authenticated open orders, spend permissions, collateral cash or private pledges.".into(),
            "Supply the actual holding/profile wallet; an empty list does not establish account ownership or zero cash.".into(),
            "Redeemable is an API hint, not a confirmed redemption or guaranteed payout; market lifecycle reconciliation is pending.".into(),
            "Offset pages are not an atomic snapshot; concurrent account changes can affect coverage. Upstream snapshot age is unknown.".into(),
        ]);
        let mut seen = HashSet::new();
        let mut count = 0usize;
        for page in 0..=20 {
            let request = Request {
                url: "https://data-api.polymarket.com/positions".into(),
                body: None,
                query: vec![
                    ("user".into(), o.account.address.as_str().into()),
                    ("sizeThreshold".into(), "0".into()),
                    ("includeArchived".into(), "true".into()),
                    ("limit".into(), "500".into()),
                    ("offset".into(), (page * 500).to_string()),
                    ("sortBy".into(), "TOKENS".into()),
                    ("sortDirection".into(), "DESC".into()),
                ],
            };
            let Some((v, e)) = self.read(o, "polymarket/positions-v1", request)? else {
                return Ok(());
            };
            let Some(rows) = v.as_array() else {
                o.issue("SCHEMA_ERROR", "positions", "Expected positions array.");
                return Ok(());
            };
            if rows.len() > 500 {
                o.issue(
                    "SCHEMA_ERROR",
                    "positions",
                    "Server exceeded requested page bound.",
                );
                return Ok(());
            }
            let parsed = parse_poly(rows, &o.account.address, &e, &mut seen);
            let failed = parsed.is_err();
            parse_into(o, "positions", parsed);
            if failed {
                return Ok(());
            }
            count += rows.len();
            if rows.len() < 500 {
                o.facts.push(fact(
                    "positions.count",
                    None,
                    FactValue::Amount(Decimal::from_atoms(count as i128, 0).unwrap()),
                    &e,
                ));
                return Ok(());
            }
        }
        o.issue(
            "PAGINATION_LIMIT",
            "positions",
            "Reached offset 10000 with a full page; remaining positions are unknown.",
        );
        Ok(())
    }
    fn rpc(
        &self,
        o: &mut AccountObservation,
        url: &str,
        method: &str,
        params: Value,
    ) -> Result<Option<(Value, EvidenceRef)>, String> {
        let endpoint = format!("evm/{method}");
        let Some((v, e)) = self.read(
            o,
            &endpoint,
            Request {
                url: url.into(),
                body: Some(json!({"jsonrpc":"2.0","id":1,"method":method,"params":params})),
                query: vec![],
            },
        )?
        else {
            return Ok(None);
        };
        if v.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || v.get("id").and_then(Value::as_u64) != Some(1)
            || v.get("result").is_none_or(Value::is_null)
        {
            o.issue(
                "SCHEMA_ERROR",
                &endpoint,
                "Missing result or mismatched RPC response identity.",
            );
            return Ok(None);
        }
        Ok(Some((v["result"].clone(), e)))
    }
    fn evm(
        &self,
        o: &mut AccountObservation,
        chain: u64,
        env: &str,
        tokens: &[Token],
    ) -> Result<(), String> {
        let Ok(url) = std::env::var(env) else {
            o.issue(
                "MISSING_RPC_CONFIG",
                "evm",
                "Set the configured HYPRSONIC_* RPC URL environment variable.",
            );
            return Ok(());
        };
        self.evm_at_url(o, chain, &url, tokens)
    }
    fn evm_at_url(
        &self,
        o: &mut AccountObservation,
        chain: u64,
        url: &str,
        tokens: &[Token],
    ) -> Result<(), String> {
        let valid = reqwest::Url::parse(url).is_ok_and(|u| {
            u.scheme() == "https"
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
                && u.fragment().is_none()
        });
        if !valid {
            o.issue(
                "INVALID_RPC_CONFIG",
                "evm",
                "RPC URL must be HTTPS without userinfo or fragment.",
            );
            return Ok(());
        }
        o.limitations.push("Pinned-block balances are not withdrawal eligibility; allowances, gas cost, external obligations and finality are not reconciled.".into());
        let Some((v, _)) = self.rpc(o, url, "eth_chainId", json!([]))? else {
            return Ok(());
        };
        if hex_integer(&v) != Ok(chain as i128) {
            o.issue(
                "CHAIN_MISMATCH",
                "evm",
                "RPC chain does not match configured asset network.",
            );
            return Ok(());
        }
        let Some((block, _)) =
            self.rpc(o, url, "eth_getBlockByNumber", json!(["latest", false]))?
        else {
            return Ok(());
        };
        let (number, hash, time) = match (
            block.get("number").and_then(Value::as_str),
            block.get("hash").and_then(Value::as_str),
            block.get("timestamp").map(hex_integer),
        ) {
            (Some(n), Some(h), Some(Ok(t)))
                if hex_integer(&Value::String(n.into())).is_ok()
                    && h.len() == 66
                    && h.starts_with("0x")
                    && h[2..].bytes().all(|b| b.is_ascii_hexdigit())
                    && t >= 0
                    && t <= (u64::MAX / 1000) as i128 =>
            {
                (n.to_owned(), h.to_owned(), t as u64 * 1000)
            }
            _ => {
                o.issue(
                    "SCHEMA_ERROR",
                    "evm/block",
                    "Block number, hash or timestamp is invalid.",
                );
                return Ok(());
            }
        };
        let address = o.account.address.as_str().to_owned();
        let network = format!("evm:{chain}");
        if let Some((v, mut e)) = self.rpc(o, url, "eth_getBalance", json!([address, number]))? {
            pin(o, &mut e, &hash, time);
            parse_into(
                o,
                "evm/native",
                hex_integer(&v)
                    .and_then(|n| Decimal::from_atoms(n, 18).map_err(str::to_owned))
                    .map(|n| {
                        vec![fact(
                            "wallet.native_balance",
                            reported_asset(&network, "native"),
                            FactValue::Amount(n),
                            &e,
                        )]
                    }),
            );
        }
        for token in tokens {
            let contract = EvmAddress::parse(&token.contract).map_err(str::to_owned)?;
            let Some((code, _)) =
                self.rpc(o, url, "eth_getCode", json!([contract.as_str(), number]))?
            else {
                continue;
            };
            if !code.as_str().is_some_and(|s| {
                s.starts_with("0x") && s.len() > 2 && s[2..].bytes().all(|b| b.is_ascii_hexdigit())
            }) {
                o.issue(
                    "TOKEN_CONTRACT_MISSING",
                    "evm/token",
                    "No valid code at the configured token address.",
                );
                continue;
            }
            let Some((decimals, _)) = self.rpc(
                o,
                url,
                "eth_call",
                json!([{"to":contract.as_str(),"data":"0x313ce567"},number]),
            )?
            else {
                continue;
            };
            if hex_integer(&decimals) != Ok(token.decimals as i128) {
                o.issue(
                    "TOKEN_DECIMALS_MISMATCH",
                    "evm/token",
                    "Onchain decimals disagree with configured precision.",
                );
                continue;
            }
            let data = format!("0x70a08231{:0>64}", &address[2..]);
            if let Some((v, mut e)) = self.rpc(
                o,
                url,
                "eth_call",
                json!([{"to":contract.as_str(),"data":data},number]),
            )? {
                pin(o, &mut e, &hash, time);
                parse_into(
                    o,
                    "evm/token",
                    hex_integer(&v)
                        .and_then(|n| Decimal::from_atoms(n, token.decimals).map_err(str::to_owned))
                        .map(|n| {
                            vec![fact(
                                &format!("wallet.token_balance.{}", token.symbol),
                                reported_asset(&network, contract.as_str()),
                                FactValue::Amount(n),
                                &e,
                            )]
                        }),
                );
            }
        }
        if let Some((end, _)) = self.rpc(o, url, "eth_getBlockByNumber", json!([number, false]))?
            && end.get("hash").and_then(Value::as_str) != Some(hash.as_str())
        {
            o.issue("BLOCK_CHANGED","evm","Pinned block was replaced or disappeared during reads; balances are not current canonical evidence.");
        }
        Ok(())
    }
}
fn pin(o: &mut AccountObservation, e: &mut EvidenceRef, hash: &str, time: u64) {
    e.block = Some(hash.into());
    e.source_time_ms = Some(time);
    if let Some(stored) = o.evidence.iter_mut().find(|x| x.id == e.id) {
        *stored = e.clone();
    }
}
pub fn hex_integer(v: &Value) -> Result<i128, String> {
    let s = v
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .ok_or("expected hex integer")?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid hex integer".into());
    }
    i128::from_str_radix(s, 16).map_err(|_| "integer exceeds supported i128 range".into())
}
pub fn parse_hl_orders(v: &Value, e: &EvidenceRef) -> Result<Vec<Fact>, String> {
    let rows = v.as_array().ok_or("expected orders array")?;
    let mut ids = HashSet::new();
    for row in rows {
        let id = row
            .get("oid")
            .and_then(Value::as_u64)
            .ok_or("missing order id")?;
        if !ids.insert(id)
            || text(row, "coin")?.is_empty()
            || !matches!(text(row, "side")?, "A" | "B")
        {
            return Err("invalid or duplicate order".into());
        }
        row.get("timestamp")
            .and_then(Value::as_u64)
            .ok_or("missing order time")?;
        if numeric(row, "sz")?.atoms() < 0 || numeric(row, "limitPx")?.atoms() < 0 {
            return Err("invalid order quantity".into());
        }
    }
    Ok(vec![fact(
        "default_dex.open_order_count",
        None,
        FactValue::Amount(Decimal::from_atoms(rows.len() as i128, 0).unwrap()),
        e,
    )])
}
pub fn parse_hl_perps(v: &Value, e: &EvidenceRef) -> Result<Vec<Fact>, String> {
    let mut facts = vec![];
    v.get("time")
        .and_then(Value::as_u64)
        .ok_or("missing account timestamp")?;
    let summary = v.get("marginSummary").ok_or("missing margin summary")?;
    for key in [
        "accountValue",
        "totalMarginUsed",
        "totalNtlPos",
        "totalRawUsd",
    ] {
        facts.push(fact(
            &format!("default_dex.reported.{key}"),
            reported_asset("hyperliquid", "reported-USD"),
            FactValue::Amount(numeric(summary, key)?),
            e,
        ));
    }
    for key in ["withdrawable", "crossMaintenanceMarginUsed"] {
        facts.push(fact(
            &format!("default_dex.reported.{key}"),
            reported_asset("hyperliquid", "reported-USD"),
            FactValue::Amount(numeric(v, key)?),
            e,
        ));
    }
    for row in list(v, "assetPositions")? {
        let p = row.get("position").ok_or("missing position")?;
        let coin = text(p, "coin")?;
        for key in ["szi", "unrealizedPnl", "marginUsed", "positionValue"] {
            let asset = if key == "szi" {
                reported_asset("hyperliquid-perp", coin)
            } else {
                reported_asset("hyperliquid", &format!("reported-USD:{coin}"))
            };
            facts.push(fact(
                &format!("position.{key}"),
                asset,
                FactValue::Amount(numeric(p, key)?),
                e,
            ));
        }
    }
    Ok(facts)
}
pub fn parse_hl_spot(v: &Value, e: &EvidenceRef) -> Result<Vec<Fact>, String> {
    let mut out = vec![];
    let mut seen = HashSet::new();
    for row in list(v, "balances")? {
        let token = row
            .get("token")
            .and_then(Value::as_u64)
            .ok_or("missing token ID")?;
        if !seen.insert(token) {
            return Err("duplicate token".into());
        }
        for key in ["total", "hold"] {
            out.push(fact(
                &format!("spot.reported.{key}"),
                reported_asset("hyperliquid-spot", &token.to_string()),
                FactValue::Amount(numeric(row, key)?),
                e,
            ));
        }
    }
    Ok(out)
}
pub fn parse_poly(
    rows: &[Value],
    address: &EvmAddress,
    e: &EvidenceRef,
    seen: &mut HashSet<String>,
) -> Result<Vec<Fact>, String> {
    let mut out = vec![];
    for row in rows {
        let wallet = EvmAddress::parse(text(row, "proxyWallet")?).map_err(str::to_owned)?;
        if &wallet != address {
            return Err("holding wallet mismatch".into());
        }
        let asset = text(row, "asset")?;
        let condition = text(row, "conditionId")?;
        if asset.is_empty()
            || asset.len() > 78
            || !asset.bytes().all(|b| b.is_ascii_digit())
            || condition.len() != 66
            || !condition.starts_with("0x")
            || !condition[2..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid instrument identity".into());
        }
        let id = format!("{}:{asset}", condition.to_ascii_lowercase());
        if !seen.insert(id.clone()) {
            return Err("duplicate position across pages; snapshot changed".into());
        }
        let size = numeric(row, "size")?;
        if size.atoms() < 0 {
            return Err("negative position size".into());
        }
        out.push(fact(
            "position.size",
            reported_asset("polymarket-predictions", &id),
            FactValue::Amount(size),
            e,
        ));
        for key in ["currentValue", "cashPnl"] {
            out.push(fact(
                &format!("position.reported.{key}"),
                reported_asset("polymarket-reported-USD", &id),
                FactValue::Amount(numeric(row, key)?),
                e,
            ));
        }
        let redeemable = row
            .get("redeemable")
            .and_then(Value::as_bool)
            .ok_or("missing redemption hint")?;
        out.push(fact(
            "position.redeemable_hint",
            reported_asset("polymarket-predictions", &id),
            FactValue::Flag(redeemable),
            e,
        ));
    }
    Ok(out)
}
impl AccountSource for Source<'_> {
    fn observe(&self) -> Result<AccountObservation, String> {
        let venue = match self.account {
            Account::Hyperliquid { .. } => "hyperliquid",
            Account::Polymarket { .. } => "polymarket",
            Account::Evm { .. } => "evm",
        };
        let mut o = AccountObservation::new(
            self.account.alias().into(),
            venue,
            EvmAddress::parse(self.account.address()).map_err(str::to_owned)?,
        );
        match self.account {
            Account::Hyperliquid { .. } => self.hl(&mut o)?,
            Account::Polymarket { .. } => self.poly(&mut o)?,
            Account::Evm {
                chain_id,
                rpc_url_env,
                tokens,
                ..
            } => self.evm(&mut o, *chain_id, rpc_url_env, tokens)?,
        }
        Ok(o)
    }
}

#[cfg(test)]
mod wallet_tests {
    use super::*;
    use crate::observe::io::{Reply, Transport};
    use std::sync::Mutex;
    const ADDRESS: &str = "0x1111111111111111111111111111111111111111";
    struct Store(Mutex<Vec<Vec<u8>>>);
    impl EvidenceStore for Store {
        fn append(&self, r: &[u8]) -> Result<String, String> {
            let mut x = self.0.lock().unwrap();
            x.push(r.to_vec());
            Ok(x.len().to_string())
        }
    }
    struct Time;
    impl Clock for Time {
        fn now_ms(&self) -> u64 {
            1000
        }
    }
    struct Rpc {
        fault: &'static str,
        calls: Mutex<Vec<String>>,
    }
    impl Transport for Rpc {
        fn send(&self, r: &Request) -> Result<Reply, String> {
            let body = r.body.as_ref().unwrap();
            let method = body["method"].as_str().unwrap();
            self.calls.lock().unwrap().push(method.into());
            let result = match method {
                "eth_chainId" => json!(if self.fault == "chain" { "0x1" } else { "0x89" }),
                "eth_getBlockByNumber" => {
                    json!({"number":"0x10","hash":format!("0x{}",if self.fault=="reorg"&&body["params"][0]!="latest"{"b".repeat(64)}else{"a".repeat(64)}),"timestamp":"0x1"})
                }
                "eth_getBalance" => {
                    assert_eq!(body["params"][1], "0x10");
                    json!("0xde0b6b3a7640000")
                }
                "eth_getCode" => json!(if self.fault == "code" { "0x" } else { "0x6000" }),
                "eth_call" => {
                    assert_eq!(body["params"][1], "0x10");
                    if body["params"][0]["data"] == "0x313ce567" {
                        json!(if self.fault == "decimals" {
                            "0x12"
                        } else {
                            "0x6"
                        })
                    } else {
                        assert_eq!(
                            body["params"][0]["data"],
                            format!("0x70a08231{:0>64}", &ADDRESS[2..])
                        );
                        json!("0xf4240")
                    }
                }
                _ => panic!("unexpected method"),
            };
            Ok(Reply {
                status: 200,
                body: serde_json::to_vec(
                    &json!({"jsonrpc":"2.0","id":if self.fault=="id"{2}else{1},"result":result}),
                )
                .unwrap(),
            })
        }
    }
    fn observe(fault: &'static str) -> AccountObservation {
        let rpc = Rpc {
            fault,
            calls: Mutex::new(vec![]),
        };
        let store = Store(Mutex::new(vec![]));
        let ctx = Context {
            transport: &rpc,
            store: &store,
            clock: &Time,
        };
        let account = Account::Evm {
            alias: "wallet".into(),
            address: ADDRESS.into(),
            chain_id: 137,
            rpc_url_env: "HYPRSONIC_TEST_RPC".into(),
            tokens: vec![],
        };
        let source = Source {
            account: &account,
            context: &ctx,
        };
        let mut o =
            AccountObservation::new("wallet".into(), "evm", EvmAddress::parse(ADDRESS).unwrap());
        source
            .evm_at_url(
                &mut o,
                137,
                "https://rpc.example.invalid/secret",
                &[Token {
                    contract: ADDRESS.into(),
                    symbol: "TEST".into(),
                    decimals: 6,
                }],
            )
            .unwrap();
        o.finish(1000, 100);
        o
    }
    #[test]
    fn wallet_reads_use_pinned_block_and_validated_precision() {
        let o = observe("");
        assert!(matches!(o.read_status, ReadStatus::Complete));
        assert_eq!(o.facts.len(), 2);
        assert!(matches!(&o.facts[1].value,FactValue::Amount(d) if d.to_string()=="1.000000"));
        for f in &o.facts {
            let e = o.evidence.iter().find(|e| e.id == f.evidence_id).unwrap();
            assert!(e.block.is_some());
            assert_eq!(e.source_time_ms, Some(1000));
        }
    }
    #[test]
    fn wrong_chain_rpc_id_contract_or_decimals_prevent_token_observation() {
        for (fault, code) in [
            ("chain", "CHAIN_MISMATCH"),
            ("id", "SCHEMA_ERROR"),
            ("code", "TOKEN_CONTRACT_MISSING"),
            ("decimals", "TOKEN_DECIMALS_MISMATCH"),
        ] {
            let o = observe(fault);
            assert!(o.issues.iter().any(|i| i.code == code));
            assert!(
                !o.facts
                    .iter()
                    .any(|f| f.field.starts_with("wallet.token_balance"))
            );
        }
    }
    #[test]
    fn a_reorg_invalidates_previously_successful_balance_reads() {
        let o = observe("reorg");
        assert_eq!(o.facts.len(), 2);
        assert!(matches!(o.read_status, ReadStatus::Partial));
        assert!(o.issues.iter().any(|i| i.code == "BLOCK_CHANGED"));
    }
}
