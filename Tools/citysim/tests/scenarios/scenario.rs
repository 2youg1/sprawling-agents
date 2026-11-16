// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
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

fn quiet_config() -> FrozenConfig {
    FrozenConfig {
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

fn probe_call() -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("probe").unwrap(),
        args: Payload::empty(),
    }
}

fn scenario(cancel: Option<CancelPoint>) -> Scenario {
    Scenario {
        run: RunId::parse("0198f6a2-7c4a-7bbb-9d1e-0000000000ff").unwrap(),
        who: "worker@sim.1".into(),
        addr: Address::parse("sim/lobby/room1").unwrap(),
        task: "close the minimal loop".into(),
        goal: "one tool wave, then stop".into(),
        job_md: "# JOB\nRun one probe, then conclude.".into(),
        model: ScriptModel::new(vec![
            ModelReturn {
                usage: None,
                stop: None,
                billed_usd_micros: None,
                message: Payload::empty(),
                calls: vec![probe_call()],
            },
            concluding("the probe answered; nothing else to do").unwrap(),
        ]),
        bench: bench_with(vec![Box::new(ScriptTool::new(
            probe_meta(),
            vec![Ok(ToolOutcome {
                result: Payload::empty(),
                attachments: Vec::new(),
            })],
        ))]),
        config: quiet_config(),
        checkpoint: None,
        cancel,
        steer: None,
        sieve: None,
    }
}

mod acceptance;
mod domain_flow;
mod loop_flow;
