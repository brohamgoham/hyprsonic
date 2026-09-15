//! Read-only application, composition root and adapters. The synthetic engine is separate.
pub mod adapters;
pub mod config;
pub mod io;
use capital_core::{AccountSource, Clock, FactValue, ReadStatus};
use serde_json::json;
use std::path::Path;

pub fn run(args: &[String]) -> Result<u8, String> {
    let mut path = ".local/accounts.json".to_owned();
    let mut json_output = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                path = args.get(i).ok_or("--config requires a path")?.clone();
            }
            "--json" => json_output = true,
            "--help" => {
                println!(
                    "hyprsonic observe [--config .local/accounts.json] [--json]\nRead-only network observations. Evidence: .local/observations/.\nExit 0: requested reads complete (not funding eligibility); 3: failed/partial/stale reads; 1: configuration/journal failure."
                );
                return Ok(0);
            }
            _ => return Err("unknown observe option".into()),
        }
        i += 1;
    }
    let config = config::Config::load(Path::new(&path))?;
    let clock = io::SystemClock;
    let started = clock.now_ms();
    let journal = io::LocalJournal::create(Path::new(".local/observations"), started)?;
    let http = io::Http::new()?;
    let context = io::Context {
        transport: &http,
        store: &journal,
        clock: &clock,
    };
    let sources: Vec<_> = config
        .accounts
        .iter()
        .map(|account| adapters::Source {
            account,
            context: &context,
        })
        .collect();
    let ports: Vec<&dyn AccountSource> = sources.iter().map(|s| s as &dyn AccountSource).collect();
    let observations = collect(&ports, &clock, config.max_age_seconds * 1000)?;
    let completed = clock.now_ms();
    let degraded = observations
        .iter()
        .any(|o| !matches!(o.read_status, ReadStatus::Complete));
    let report = json!({"schema_version":1,"mode":"live_read_only","adapter_version":"phase1-v1","started_at_ms":started,
        "completed_at_ms":completed,"funding_verdict":"UNDETERMINED","capital_reconciled":false,
        "scope":"Configured accounts and endpoints only. No signing, execution or funding eligibility.","accounts":observations});
    journal.report(&report)?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|_| "report encoding failed")?
        );
    } else {
        println!(
            "HyprSonic — live account observations\nFunding verdict: UNDETERMINED (Phase 1; capital is not reconciled)"
        );
        for o in &observations {
            println!(
                "\n{} [{}] {:?} | mode={:?} | {} facts / {} responses",
                o.alias,
                o.account.venue,
                o.read_status,
                o.account_mode.as_deref().unwrap_or("not reported"),
                o.facts.len(),
                o.evidence.len()
            );
            for f in o.facts.iter().take(12) {
                let value = match &f.value {
                    FactValue::Amount(a) => a.to_string(),
                    FactValue::Text(s) => format!("{s:?}"),
                    FactValue::Flag(b) => b.to_string(),
                };
                println!("  {} = {} [{}]", f.field, value, f.evidence_id);
            }
            if o.facts.len() > 12 {
                println!("  ... all facts are in the private report / --json output");
            }
            for issue in &o.issues {
                println!("  ! {}: {} — {}", issue.code, issue.scope, issue.detail);
            }
            for limit in &o.limitations {
                println!("  coverage: {limit}");
            }
        }
        println!("\nPrivate evidence: {}", journal.path.display());
    }
    eprintln!("observation journal: {}", journal.path.display());
    Ok(if degraded { 3 } else { 0 })
}

/// Application use case: depends only on account/clock ports, not concrete adapters.
pub fn collect(
    sources: &[&dyn AccountSource],
    clock: &dyn Clock,
    max_age_ms: u64,
) -> Result<Vec<capital_core::AccountObservation>, String> {
    if sources.is_empty() || sources.len() > 16 || max_age_ms == 0 {
        return Err("invalid observation scope".into());
    }
    let mut observations = vec![];
    for batch in sources.chunks(4) {
        let results = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|source| scope.spawn(move || source.observe()))
                .collect();
            workers
                .into_iter()
                .map(|w| {
                    w.join()
                        .map_err(|_| "observation worker failed".to_owned())
                        .and_then(|r| r)
                })
                .collect::<Vec<_>>()
        });
        for result in results {
            observations.push(result?);
        }
    }
    let completed = clock.now_ms();
    for o in &mut observations {
        o.finish(completed, max_age_ms);
    }
    Ok(observations)
}
