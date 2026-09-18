//! Capital application use case and CLI; the pure core never sees transport JSON.
use crate::observe::{
    self,
    adapters::{CapitalSource, Source},
    config::Config,
    io::{Context, Http, LocalJournal, SystemClock},
};
use capital_core::capital::{CapitalReport, ReconcilePolicy, Reservation, reconcile};
use capital_core::{AccountSource, Clock, Decimal};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyFile {
    reservations: Vec<ReservationInput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReservationInput {
    id: String,
    claim_id: String,
    amount: String,
}
pub fn load_reservations(path: &Path) -> Result<Vec<Reservation>, String> {
    let bytes = std::fs::read(path).map_err(|_| "cannot read local reservation policy")?;
    if bytes.len() > 65536 {
        return Err("policy exceeds 64 KiB".into());
    }
    let file: PolicyFile =
        serde_json::from_slice(&bytes).map_err(|_| "invalid reservation policy schema")?;
    if file.reservations.len() > 256 {
        return Err("at most 256 local reservations".into());
    }
    file.reservations
        .into_iter()
        .map(|r| {
            if r.id.is_empty()
                || r.id.len() > 64
                || !r
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || r.claim_id.len() > 512
            {
                return Err("invalid reservation identity".into());
            }
            Ok(Reservation {
                id: r.id,
                claim_id: r.claim_id,
                amount: Decimal::parse(&r.amount).map_err(str::to_owned)?,
            })
        })
        .collect()
}
pub fn run(args: &[String]) -> Result<u8, String> {
    let mut config_path = ".local/accounts.json".to_owned();
    let mut policy_path = None;
    let mut json_output = false;
    let mut explain = false;
    let mut max_skew_seconds = 30u64;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_output = true,
            "--explain" => explain = true,
            "--help" => {
                println!(
                    "hyprsonic capital [--config .local/accounts.json] [--policy .local/reserves.json] [--max-skew-seconds 30] [--json] [--explain]\nLive capital projections, finalized wallet reads and versioned constraints. No signing.\nExit 0: scoped reconciliation completed; 3: evidence/rule/reservation gaps or local margin failure; 1: invalid configuration/journal. Neither 0 nor 3 is funding approval."
                );
                return Ok(0);
            }
            flag @ ("--config" | "--policy" | "--max-skew-seconds") => {
                i += 1;
                let value = args.get(i).ok_or("option requires a value")?;
                match flag {
                    "--config" => config_path = value.clone(),
                    "--policy" => policy_path = Some(value.clone()),
                    _ => max_skew_seconds = value.parse().map_err(|_| "invalid skew bound")?,
                }
            }
            _ => return Err("unknown capital option".into()),
        }
        i += 1;
    }
    if !(1..=3600).contains(&max_skew_seconds) {
        return Err("skew bound must be 1..3600 seconds".into());
    }
    let reservations = policy_path
        .map(|p| load_reservations(Path::new(&p)))
        .transpose()?
        .unwrap_or_default();
    let config = Config::load(Path::new(&config_path))?;
    let clock = SystemClock;
    let started = clock.now_ms();
    let journal = LocalJournal::create(Path::new(".local/capital"), started)?;
    let http = Http::new()?;
    let context = Context {
        transport: &http,
        store: &journal,
        clock: &clock,
    };
    let sources: Vec<_> = config
        .accounts
        .iter()
        .map(|account| {
            CapitalSource(Source {
                account,
                context: &context,
            })
        })
        .collect();
    let ports: Vec<&dyn AccountSource> = sources.iter().map(|s| s as &dyn AccountSource).collect();
    let observations = observe::collect(&ports, &clock, config.max_age_seconds * 1000)?;
    let completed = clock.now_ms();
    let policy = ReconcilePolicy {
        now_ms: completed,
        max_age_ms: config.max_age_seconds * 1000,
        max_skew_ms: max_skew_seconds * 1000,
        reservations,
    };
    let timer = Instant::now();
    let capital = reconcile(&observations, &policy);
    let core_us = timer.elapsed().as_micros();
    let report = json!({"schema_version":2,"mode":"live_capital_read_only","adapter_version":observe::io::ADAPTER_VERSION,
        "started_at_ms":started,"completed_at_ms":completed,"collection_elapsed_ms":completed.saturating_sub(started),
        "reconciliation_elapsed_us":core_us,"capital":capital,"accounts":observations,"reservations":policy.reservations});
    journal.report(&report)?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|_| "report encoding failed")?
        );
    } else {
        print_capital(&capital, explain);
        println!(
            "\nCollection: {} ms | core reconciliation: {} µs\nPrivate report: {}/report.json",
            completed.saturating_sub(started),
            core_us,
            journal.path.display()
        );
    }
    eprintln!("capital journal: {}", journal.path.display());
    Ok(if capital.issues.is_empty() { 0 } else { 3 })
}
fn show(p: &capital_core::capital::Projection) -> String {
    p.amount
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or("UNKNOWN".into())
}
fn print_capital(r: &CapitalReport, explain: bool) {
    println!(
        "HyprSonic — live capital states\nFunding verdict: {}\nViews overlap; NEVER SUM. Amounts retain each claim's asset units.",
        r.funding_verdict
    );
    println!(
        "{} holdings | {} reconciliation issues | rules {}",
        r.claims.len(),
        r.issues.len(),
        r.rule_version
    );
    for c in r.claims.iter().take(if explain { usize::MAX } else { 20 }) {
        println!(
            "\n{} [{} / {}] {}\n  claim: {}\n  observed={} | settled={} | withdrawable={} | unrealized={}\n  pending={} | pledged/reserved={} | delayed/blocked={}\n  local reserves={} | reported ceiling after local reserves={}",
            c.account_alias,
            c.asset.network,
            c.asset.id,
            c.kind,
            c.id,
            show(&c.observed_quantity),
            show(&c.states.settled),
            show(&c.states.withdrawable),
            show(&c.states.unrealized),
            show(&c.states.pending_proceeds),
            show(&c.states.pledged_reserved),
            show(&c.states.delayed_blocked),
            show(&c.local_reserved),
            show(&c.after_local_reserves)
        );
        if let Some(m) = &c.margin {
            println!(
                "  local margin: equity={} initial={} maintenance={} buffer={}",
                m.equity, m.initial_required, m.maintenance_required, m.maintenance_buffer
            );
        }
        println!("  blockers: {}", c.blockers.join(", "));
        for a in &c.annotations {
            println!("  {a}");
        }
    }
    if !explain && r.claims.len() > 20 {
        println!(
            "\n... {} more holdings in private report / --json / --explain",
            r.claims.len() - 20
        );
    }
    for i in &r.issues {
        println!("! [{}] {}: {}", i.code, i.scope, i.detail);
    }
    if explain {
        for entry in &r.ledger {
            println!(
                "#{} [{}] {} — {} [{}]",
                entry.sequence,
                entry.kind,
                entry.claim_id.as_deref().unwrap_or("account"),
                entry.explanation,
                entry.evidence.join(", ")
            );
        }
    }
}
/// Inspect persisted classifications without refetching or presenting them as current.
pub fn explain(args: &[String]) -> Result<u8, String> {
    if args == ["--help"] {
        println!(
            "hyprsonic explain --report .local/capital/RUN/report.json [--claim CLAIM_ID]\nInspect saved evidence/rule references. Does not assert that historical capital is current."
        );
        return Ok(0);
    }
    let mut path = None;
    let mut claim = None;
    let mut i = 0;
    while i < args.len() {
        let flag = &args[i];
        i += 1;
        let value = args.get(i).ok_or("explain option requires a value")?;
        match flag.as_str() {
            "--report" => path = Some(value),
            "--claim" => claim = Some(value),
            _ => return Err("unknown explain option".into()),
        }
        i += 1;
    }
    let data = std::fs::read(path.ok_or("explain requires --report")?)
        .map_err(|_| "cannot read saved capital report")?;
    if data.len() > 32 * 1024 * 1024 {
        return Err("report exceeds 32 MiB".into());
    }
    let report: Value = serde_json::from_slice(&data).map_err(|_| "invalid report JSON")?;
    if report["schema_version"] != 2 || report["mode"] != "live_capital_read_only" {
        return Err("not a Phase 2 capital report".into());
    }
    let capital = &report["capital"];
    let entries = capital["ledger"]
        .as_array()
        .ok_or("report missing ledger")?;
    if let Some(id) = claim
        && !capital["claims"]
            .as_array()
            .is_some_and(|v| v.iter().any(|c| c["id"].as_str() == Some(id)))
    {
        return Err("claim not found in report".into());
    }
    println!(
        "HISTORICAL REPORT — not current funding availability\nRules: {} | evaluated at: {}",
        capital["rule_version"], capital["evaluated_at_ms"]
    );
    for entry in entries
        .iter()
        .filter(|e| claim.is_none_or(|id| e["claim_id"].as_str() == Some(id)))
    {
        println!(
            "{}",
            serde_json::to_string_pretty(entry).map_err(|_| "ledger encoding failed")?
        );
    }
    println!(
        "Issues: {}",
        serde_json::to_string_pretty(&capital["issues"]).map_err(|_| "issue encoding failed")?
    );
    Ok(0)
}
