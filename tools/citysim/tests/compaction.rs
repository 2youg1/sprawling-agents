// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A wave that crosses the exchange's budget mid-flight: the ledger
//! keeps every byte it wrote, and the same script on a counted clock
//! replays byte for byte.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use citysim::{Scenario, ScriptModel, ScriptTool, concluding, run_scenario};
use kernel::{
    Address, ClockStampGranularity, ContentBlock, CostTier, Effect, FrozenConfig, ModelReturn,
    Payload, RenderIntent, RunId, Temporal, ToolCall, ToolMeta, ToolName, ToolOutcome, WriteDomain,
};
use runtime::bench::ToolBench;

/// Deterministic prose of at least `len` bytes, cut on a word end.
fn prose(len: usize) -> String {
    let unit = "lorem ipsum dolor sit amet ";
    let mut out = unit.repeat(len / unit.len() + 1);
    out.truncate(len);
    out
}

fn bench_with(tools: Vec<Box<dyn kernel::Tool>>) -> ToolBench {
    let domain = WriteDomain::new(vec![Address::parse("sim/lobby/room1").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    for tool in tools {
        bench.register(tool).unwrap();
    }
    bench
}

fn quiet_config() -> FrozenConfig {
    FrozenConfig {
        clock_stamp: ClockStampGranularity::Off,
        clock_zones: Vec::new(),
        sandbox: kernel::SandboxLimits::default(),
        mcp: Vec::new(),
        effort: None,
        second_threshold: None,
        search: kernel::config::SearchConfiguration::Default,
    }
}

fn probe_meta() -> ToolMeta {
    ToolMeta {
        name: ToolName::parse("probe").unwrap(),
        disclosure: "scripted probe; call it when the scenario says so".into(),
        params: Payload::empty(),
        effect: Effect::Read,
        cost_tier: CostTier::Free,
        timeout: None,
        render: RenderIntent::Generic,
        temporal: Temporal::Timeless,
    }
}

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("probe").unwrap(),
        args: Payload::empty(),
    }
}

fn loud_result() -> ToolOutcome {
    ToolOutcome {
        result: Payload::of(&serde_json::json!({ "note": prose(15_000) })).unwrap(),
        attachments: Vec::new(),
    }
}

/// One reply and one wave whose first result already pushes the
/// exchange past its budget; then a reply that concludes.
fn scenario() -> Scenario {
    let reply = prose(20_000);
    Scenario {
        run: RunId::parse("0198f6a2-7c4a-7bbb-9d1e-0000000000ff").unwrap(),
        who: "worker@sim.1".into(),
        addr: Address::parse("sim/lobby/room1").unwrap(),
        task: "cross the budget mid-wave".into(),
        goal: "one over-budget wave, then stop".into(),
        job_md: "# JOB\nRun two probes, then conclude.".into(),
        model: ScriptModel::new(vec![
            ModelReturn::bare(
                kernel::model::message_payload(&[ContentBlock::Text { text: reply }]).unwrap(),
                vec![call("call-1"), call("call-2")],
            ),
            concluding("the probes answered; nothing else to do").unwrap(),
        ]),
        bench: bench_with(vec![Box::new(ScriptTool::new(
            probe_meta(),
            vec![Ok(loud_result()), Ok(loud_result())],
        ))]),
        config: quiet_config(),
        checkpoint: None,
        cancel: None,
        steer: None,
        sieve: None,
    }
}

#[test]
fn a_wave_that_crosses_the_budget_mid_flight_keeps_the_ledger_whole() {
    let report = run_scenario(scenario()).unwrap();
    let reply = prose(20_000);
    let mut results = 0u32;
    let mut models = 0u32;
    for line in &report.lines {
        let written = String::from_utf8_lossy(line).into_owned();
        let value: serde_json::Value = serde_json::from_str(&written).unwrap();
        match value["kind"].as_str().unwrap() {
            "model_returned" => {
                models += 1;
                if models == 1 {
                    assert!(written.contains(&reply), "the ledger kept the full reply");
                }
            }
            "tool_result" => {
                results += 1;
                if !written.contains(&prose(15_000)) {
                    let head = written.get(..400).unwrap_or(&written);
                    panic!("missing result; head: {head}");
                }
            }
            _ => {}
        }
    }
    assert_eq!(results, 2);
    citysim::check_chain(report.lines).unwrap();
}

#[test]
fn the_same_script_replays_byte_for_byte() {
    let one = run_scenario(scenario()).unwrap();
    let two = run_scenario(scenario()).unwrap();
    assert_eq!(one.lines, two.lines, "same scenario, same bytes");
}
