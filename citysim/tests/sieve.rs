// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! card-11.4: the sieve under the scenario driver. The same seed and
//! the same filter table replay a byte-identical window, the second
//! call of one command shows only what changed, and every sieved
//! result carries the way back to its original.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use citysim::{Scenario, ScriptModel, ScriptTool, SieveWorld, run_scenario};
use kernel::{
    Address, ClockStampGranularity, CostTier, Effect, FrozenConfig, ModelReturn, Payload,
    RenderIntent, RunId, Temporal, ToolCall, ToolMeta, ToolName, ToolOutcome, WriteDomain,
};
use runtime::FilterTable;
use runtime::bench::ToolBench;
use serde_json::{Map, Value, json};

fn exec_meta() -> ToolMeta {
    ToolMeta {
        name: ToolName::parse("exec").unwrap(),
        disclosure: "scripted exec".into(),
        params: Payload::empty(),
        effect: Effect::Read,
        cost_tier: CostTier::Heavy,
        timeout: None,
        render: RenderIntent::Terminal,
        temporal: Temporal::Timestamped,
    }
}

fn cargo_call(id: &str) -> ToolCall {
    let mut args = Map::new();
    args.insert(
        "arm".to_owned(),
        json!({"program": {"path": "cargo", "args": ["check"]}}),
    );
    ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("exec").unwrap(),
        args: Payload::new(args).unwrap(),
    }
}

fn cargo_output(extra: &str) -> ToolOutcome {
    let mut stdout = String::new();
    for n in 0..150 {
        stdout.push_str(&format!("\x1b[32m    Checking\x1b[0m crate{n} v0.{n}.0\n"));
    }
    let mut stderr = String::new();
    for n in 0..8 {
        stderr.push_str(&format!(
            "warning: unused variable: `v{n}`\n --> src/m{n}.rs:{n}:5\n  |\n{n} |     let v{n} = 0;\n  |\n\n"
        ));
    }
    stderr.push_str(extra);
    let mut result = Map::new();
    result.insert("arm".to_owned(), Value::String("program".to_owned()));
    result.insert("stdout".to_owned(), Value::String(stdout));
    result.insert("stderr".to_owned(), Value::String(stderr));
    result.insert("exit_code".to_owned(), Value::Number(0.into()));
    ToolOutcome {
        result: Payload::new(result).unwrap(),
        attachments: Vec::new(),
    }
}

fn turn(calls: Vec<ToolCall>) -> ModelReturn {
    ModelReturn {
        usage: None,
        stop: None,
        billed_usd_micros: None,
        message: Payload::empty(),
        calls,
    }
}

fn scenario(root: &std::path::Path) -> Scenario {
    let domain = WriteDomain::new(vec![Address::parse("sim/lobby/room1").unwrap()]).unwrap();
    let mut bench = ToolBench::new(domain);
    bench
        .register(Box::new(ScriptTool::new(
            exec_meta(),
            vec![
                Ok(cargo_output("")),
                Ok(cargo_output(
                    "error[E0425]: cannot find value `gone` in this scope\n --> src/new.rs:4:9\n",
                )),
            ],
        )))
        .unwrap();
    Scenario {
        run: RunId::parse("0198f6a2-7c4a-7bbb-9d1e-00000000c11e").unwrap(),
        who: "worker@sim.1".into(),
        addr: Address::parse("sim/lobby/room1").unwrap(),
        task: "check twice".into(),
        goal: "read the sieve".into(),
        job_md: "# JOB\nRun cargo check twice.".into(),
        model: ScriptModel::new(vec![
            turn(vec![cargo_call("call-1")]),
            turn(vec![cargo_call("call-2")]),
            turn(vec![]),
        ]),
        bench,
        config: FrozenConfig {
            clock_stamp: ClockStampGranularity::Off,
            clock_zones: Vec::new(),
            sandbox: kernel::SandboxLimits::default(),
            mcp: Vec::new(),
            effort: None,
        },
        checkpoint: None,
        cancel: None,
        steer: None,
        sieve: Some(SieveWorld::open(root, FilterTable::builtin()).unwrap()),
    }
}

fn tool_results(lines: &[Vec<u8>]) -> Vec<Value> {
    lines
        .iter()
        .map(|line| serde_json::from_slice::<Value>(line).unwrap())
        .filter(|value| value["kind"] == "tool_result")
        .collect()
}

#[test]
fn the_same_seed_and_table_replay_a_byte_identical_window() {
    let dir = tempfile::tempdir().unwrap();
    let first = run_scenario(scenario(dir.path())).unwrap();
    let second = run_scenario(scenario(dir.path())).unwrap();
    assert_eq!(first.completion, "done");
    assert_eq!(first.lines, second.lines, "determinism is the whole point");
}

#[test]
fn the_window_holds_the_diagnostics_and_the_way_back_and_only_the_news_the_second_time() {
    let dir = tempfile::tempdir().unwrap();
    let report = run_scenario(scenario(dir.path())).unwrap();
    let results = tool_results(&report.lines);
    assert_eq!(results.len(), 2);
    let first = results[0]["data"]["result"]["content"].as_str().unwrap();
    assert!(first.contains("warning: unused variable: `v7`"), "{first}");
    assert!(!first.contains("Checking"), "{first}");
    assert!(!first.contains("\x1b["), "{first}");
    assert!(first.contains("filter=cargo"), "{first}");
    let account = &results[0]["data"]["result"]["sieve"][0];
    let original = account["original"].as_str().unwrap();
    assert!(original.starts_with("cas:b3-"));
    assert!(std::path::Path::new(account["rest_path"].as_str().unwrap()).exists());
    let second = results[1]["data"]["result"]["content"].as_str().unwrap();
    assert!(second.contains("error[E0425]"), "{second}");
    assert!(second.contains("[unchanged:"), "{second}");
    assert!(
        !second.contains("warning: unused variable: `v7`"),
        "{second}"
    );
    assert!(second.len() < first.len());
}
