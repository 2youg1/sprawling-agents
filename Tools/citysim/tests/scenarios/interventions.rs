// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use citysim::{CancelPoint, Scenario, ScriptModel, ScriptTool, concluding, run_scenario};
use kernel::{
    Address, ClockStampGranularity, CostTier, Effect, FrozenConfig, ModelReturn, Payload,
    RenderIntent, RunId, Temporal, ToolCall, ToolMeta, ToolName, ToolOutcome, WriteDomain,
};
use runtime::bench::ToolBench;

fn bench_with(tools: Vec<Box<dyn kernel::Tool>>) -> ToolBench {
    let domain = WriteDomain::new(vec![Address::parse("sim/lobby/room1").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    for tool in tools {
        bench.register(tool).unwrap();
    }
    bench
}

fn probe_meta() -> ToolMeta {
    ToolMeta {
        name: ToolName::parse("probe").unwrap(),
        disclosure: "scripted probe".into(),
        params: Payload::empty(),
        effect: Effect::Read,
        cost_tier: CostTier::Free,
        timeout: None,
        render: RenderIntent::Generic,
        temporal: Temporal::Timeless,
    }
}

fn probe_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("probe").unwrap(),
        args: Payload::empty(),
    }
}

fn answer(calls: Vec<ToolCall>) -> ModelReturn {
    ModelReturn {
        usage: None,
        stop: None,
        billed_usd_micros: None,
        message: Payload::empty(),
        calls,
    }
}

fn kinds(lines: &[Vec<u8>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_slice(line).unwrap();
            value["kind"].as_str().unwrap().to_owned()
        })
        .collect()
}

fn two_wave_scenario(cancel: Option<CancelPoint>, steer: Option<(u32, String)>) -> Scenario {
    Scenario {
        run: RunId::parse("0198f6a2-7c4a-7bbb-9d1e-00000000040a").unwrap(),
        who: "worker@sim.1".into(),
        addr: Address::parse("sim/lobby/room1").unwrap(),
        task: "walk two waves".into(),
        goal: "let an intervention land between them".into(),
        job_md: "# JOB\nTwo probes, then conclude.".into(),
        model: ScriptModel::new(vec![
            answer(vec![probe_call("call-1")]),
            answer(vec![probe_call("call-2")]),
            concluding("two waves walked").unwrap(),
        ]),
        bench: bench_with(vec![Box::new(ScriptTool::new(
            probe_meta(),
            vec![
                Ok(ToolOutcome {
                    result: Payload::empty(),
                    attachments: Vec::new(),
                }),
                Ok(ToolOutcome {
                    result: Payload::empty(),
                    attachments: Vec::new(),
                }),
            ],
        ))]),
        config: FrozenConfig {
            clock_stamp: ClockStampGranularity::Off,
            clock_zones: Vec::new(),
            sandbox: kernel::SandboxLimits::default(),
            mcp: Vec::new(),
            mcp_references: Vec::new(),
            skill_pins: std::collections::BTreeMap::new(),
            effort: None,
            second_threshold: None,
            search: kernel::config::SearchConfiguration::Default,
            catalog: kernel::config::CatalogExposure::default(),
        },
        checkpoint: None,
        cancel,
        steer,
        sieve: None,
    }
}

#[test]
fn a_steer_advances_the_run_instead_of_stopping_it() {

    let steered = run_scenario(two_wave_scenario(
        None,
        Some((0, "try the other branch".to_owned())),
    ))
    .unwrap();
    assert_eq!(steered.completion, "done");

    let names = kinds(&steered.lines);
    assert!(
        names.iter().any(|k| k == "steer_received"),
        "the steer is on the record: {names:?}"
    );
    assert!(
        names.iter().filter(|k| *k == "tool_called").count() >= 2,
        "both waves still ran: {names:?}"
    );
    assert_eq!(
        names.last().map(String::as_str),
        Some("run_frozen"),
        "and the Run reached its own conclusion"
    );
}

#[test]
fn a_cancel_at_the_same_boundary_beats_a_steer() {

    let both = run_scenario(two_wave_scenario(
        Some(CancelPoint::BeforeWave { turn: 0 }),
        Some((0, "keep going".to_owned())),
    ))
    .unwrap();
    assert_eq!(both.completion, "cancelled");
    let names = kinds(&both.lines);
    assert!(
        !names.iter().any(|k| k == "steer_received"),
        "the steer never landed: {names:?}"
    );
}

#[test]
fn a_steered_run_is_still_byte_identical_on_a_second_pass() {

    let steer = || Some((1, "narrow the search".to_owned()));
    let first = run_scenario(two_wave_scenario(None, steer())).unwrap();
    let second = run_scenario(two_wave_scenario(None, steer())).unwrap();
    assert_eq!(first.lines, second.lines);
}

#[test]
fn a_steer_after_the_last_wave_never_lands_and_changes_nothing() {

    let plain = run_scenario(two_wave_scenario(None, None)).unwrap();
    let late = run_scenario(two_wave_scenario(None, Some((99, "too late".to_owned())))).unwrap();
    assert_eq!(plain.lines, late.lines);
}

#[test]
fn the_chain_still_verifies_after_every_intervention() {
    for (cancel, steer) in [
        (None, Some((0, "steer".to_owned()))),
        (Some(CancelPoint::BeforeWave { turn: 1 }), None),
        (
            Some(CancelPoint::BeforeCall { turn: 1 }),
            Some((0, "steer".to_owned())),
        ),
    ] {
        let report = run_scenario(two_wave_scenario(cancel, steer)).unwrap();
        citysim::check_chain(report.lines.clone())
            .expect("an intervened Run still verifies offline");
    }
}

#[test]
fn golden_p0_pins_a_whole_intervened_run() {
    let report = run_scenario(two_wave_scenario(
        None,
        Some((0, "narrow the search to the changed files".to_owned())),
    ))
    .unwrap();

    let mut bytes: Vec<u8> = Vec::new();
    for line in &report.lines {
        bytes.extend_from_slice(line);
        bytes.push(b'\n');
    }

    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("golden-p0")
        .join("ledger-00000000000000000000.jsonl");
    if std::env::var("GOLDEN_WRITE").as_deref() == Ok("1") {
        std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
        std::fs::write(&fixture, &bytes).unwrap();
    }
    let committed = std::fs::read(&fixture).expect("run once with GOLDEN_WRITE=1");
    assert_eq!(
        committed, bytes,
        "a whole P0 loop must materialize byte-identically on every OS"
    );
    citysim::check_chain(report.lines).expect("and it verifies offline");
}
