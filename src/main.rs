use anyhow::{Context, Result, bail, ensure};
use hyprsonic::{engine::replay, model::*};
use std::{env, path::PathBuf, process::ExitCode};

fn capital(s: &Snapshot) {
    println!("Capital states (cents rendered as synthetic USD; columns overlap, DO NOT SUM):");
    println!(
        "{:<13} {:>12} {:>12} {:>12} {:>12} {:>14} {:>15}",
        "Location",
        "Settled",
        "Withdrawable",
        "Unrealized",
        "Pending",
        "Pledged/resvd",
        "Delayed/blocked"
    );
    for v in &s.capital {
        println!(
            "{:<13} {:>12} {:>12} {:>12} {:>12} {:>14} {:>15}",
            format!("{:?}", v.venue),
            dollars(v.settled_cents),
            dollars(v.withdrawable_cents),
            dollars(v.unrealized_cents),
            dollars(v.pending_proceeds_cents),
            dollars(v.pledged_reserved_cents),
            dollars(v.delayed_blocked_cents)
        );
    }
    println!(
        "Marked equity={} | loan liability={} | costs={} | HL local={} required={}",
        dollars(s.aggregate_equity_cents),
        dollars(s.loan_liability_cents),
        dollars(s.total_cost_cents),
        dollars(s.hl_local_equity_cents),
        dollars(s.hl_required_cents)
    );
}

fn text_report(r: &Report) {
    println!("HYPRSONIC LANE 1 PHASE 0 — SYNTHETIC, OFFLINE, NO EXECUTION");
    println!(
        "Fixture={} | plan={} | scenario={} | horizon={} ticks",
        r.fixture_id,
        r.plan.name(),
        r.scenario.name(),
        r.fixture.horizon
    );
    capital(&r.ledger[0].state);
    for e in &r.ledger {
        println!(
            "#{:03} t={} [{}] {}",
            e.sequence, e.tick, e.code, e.explanation
        );
        println!(
            "      HL {}/{} | aggregate {} | costs {} | liability {}",
            dollars(e.state.hl_local_equity_cents),
            dollars(e.state.hl_required_cents),
            dollars(e.state.aggregate_equity_cents),
            dollars(e.state.total_cost_cents),
            dollars(e.state.loan_liability_cents)
        );
    }
    capital(&r.final_state);
    println!(
        "RESULT={} (selected horizon only)",
        if r.feasible { "SURVIVES" } else { "FAIL" }
    );
}

fn demo(f: &Fixture) -> Result<()> {
    println!("HYPRSONIC LANE 1 PHASE 0 — SYNTHETIC CAPITAL PLAN DEMO");
    println!(
        "Question: can this proposed change be funded and maintained under the selected scenario?"
    );
    println!(
        "Action: {}. No live adapter, real financing offer, or trading authority.",
        f.action.description
    );
    println!("Both scenarios use the SAME price move; only Polymarket settlement timing changes.");
    println!(
        "Kalshi: close={} determination={} settlement={}; commercial access UNMET.",
        f.rules.kalshi_close_at, f.rules.kalshi_determination_at, f.rules.kalshi_settlement_at
    );
    let mut reports = vec![];
    for scenario in [Scenario::OnTime, Scenario::Delayed] {
        for plan in Plan::ALL {
            reports.push(replay(f, plan, scenario, false)?);
        }
    }
    capital(&reports[0].ledger[0].state);
    println!("\nPlans are independent alternatives; no collateral is shared between replays.");
    println!(
        "All plans sweep Polymarket proceeds ONLY after confirmed settlement plus transfer delay."
    );
    println!(
        "{:<10} {:<10} {:<10} {:>10} {:>12} {:>12} {:>12}",
        "Scenario", "Plan", "Outcome", "Cost", "HL local", "HL required", "Liability"
    );
    for r in &reports {
        let s = &r.final_state;
        println!(
            "{:<10} {:<10} {:<10} {:>10} {:>12} {:>12} {:>12}",
            r.scenario.name(),
            r.plan.name(),
            if r.feasible { "SURVIVES" } else { "FAIL" },
            dollars(s.total_cost_cents),
            dollars(s.hl_local_equity_cents),
            dollars(s.hl_required_cents),
            dollars(s.loan_liability_cents)
        );
    }
    let failed = reports
        .iter()
        .find(|r| r.plan == Plan::Wallet && r.scenario == Scenario::Delayed)
        .unwrap();
    let breach = failed.ledger.iter().find(|e| e.code == "HL_MARGIN_UNMET");
    ensure!(
        reports[0].feasible && breach.is_some(),
        "fixture does not demonstrate the required on-time success / delayed local-margin failure"
    );
    let breach = breach.unwrap();
    println!(
        "\nFAIL MOMENT — ledger #{} at t={}: {}",
        breach.sequence, breach.tick, breach.explanation
    );
    let cheapest_baseline = reports
        .iter()
        .filter(|r| r.scenario == Scenario::OnTime && r.feasible)
        .map(|r| r.final_state.total_cost_cents)
        .min()
        .context("no baseline funding plan")?;
    let alternatives: Vec<_> = reports
        .iter()
        .filter(|r| r.scenario == Scenario::Delayed && r.feasible)
        .collect();
    ensure!(
        !alternatives.is_empty(),
        "no feasible alternative in selected scenario"
    );
    println!("\nFEASIBLE ALTERNATIVES (cost delta vs cheapest successful on-time plan):");
    for r in alternatives {
        println!(
            "  {}: cost {}; delta {}; local margin buffer {}",
            r.plan.name(),
            dollars(r.final_state.total_cost_cents),
            dollars(r.final_state.total_cost_cents - cheapest_baseline),
            dollars(r.final_state.hl_local_equity_cents - r.final_state.hl_required_cents)
        );
    }
    println!("  reduce changes existing exposure; it keeps the proposed action unchanged.");
    println!(
        "  finance pledges wallet + Polymarket settled lots; outstanding debt and maturity remain visible."
    );
    println!("\nNON-ATOMIC FAILURE — break the finance disbursement after collateral pledge:");
    let broken = replay(f, Plan::Finance, Scenario::Delayed, true)?;
    ensure!(!broken.feasible, "injected failure unexpectedly succeeded");
    for e in broken.ledger.iter().filter(|e| {
        e.code == "COLLATERAL_PLEDGED"
            || e.code == "FINANCING_NOT_DELIVERED"
            || e.code == "ACTION_NOT_EXECUTED"
    }) {
        println!(
            "  #{} t={} [{}] {}",
            e.sequence, e.tick, e.code, e.explanation
        );
    }
    println!(
        "\nSurvival covers this synthetic horizon only, not eventual loan repayment or liquidation execution."
    );
    println!(
        "Inspect the full ledger: cargo run --offline -- replay --scenario delayed --plan wallet"
    );
    Ok(())
}

fn run() -> Result<u8> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "observe") {
        return hyprsonic::observe::run(&args[1..]).map_err(anyhow::Error::msg);
    }
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        println!(
            "hyprsonic observe [--config .local/accounts.json] [--json]\nObserve: read-only network access; exit 3 for incomplete reads.\n\nhyprsonic demo [--fixture PATH]\nhyprsonic replay --scenario on-time|delayed --plan wallet|withdraw|reduce|finance [--fixture PATH] [--break-step] [--json]\nExit: 0 successful demo/feasible plan, 2 infeasible plan, 1 invalid input. Demo/replay are offline and synthetic."
        );
        return Ok(0);
    }
    let command = args[0].as_str();
    ensure!(
        matches!(command, "demo" | "replay"),
        "unknown command {command}"
    );
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/lane1.json");
    let mut scenario = None;
    let mut plan = None;
    let mut json = false;
    let mut broken = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" if command == "replay" => json = true,
            "--break-step" if command == "replay" => broken = true,
            flag @ ("--fixture" | "--scenario" | "--plan") => {
                i += 1;
                let value = args
                    .get(i)
                    .with_context(|| format!("missing value for {flag}"))?;
                match flag {
                    "--fixture" => path = value.into(),
                    "--scenario" if command == "replay" => scenario = Some(Scenario::parse(value)?),
                    "--plan" if command == "replay" => plan = Some(Plan::parse(value)?),
                    _ => bail!("{flag} is only valid for replay"),
                }
            }
            flag => bail!("unknown option {flag}"),
        }
        i += 1;
    }
    let f = Fixture::load(&path).with_context(|| format!("invalid fixture {}", path.display()))?;
    if command == "demo" {
        demo(&f)?;
        return Ok(0);
    }
    let r = replay(
        &f,
        plan.context("replay requires --plan")?,
        scenario.context("replay requires --scenario")?,
        broken,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&r)?);
    } else {
        text_report(&r);
    }
    Ok(if r.feasible { 0 } else { 2 })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("FAIL: {e:#}");
            ExitCode::FAILURE
        }
    }
}
