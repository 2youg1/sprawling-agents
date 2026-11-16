// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn the_minimal_loop_closes_and_the_chain_verifies() {
    let report = run_scenario(scenario(None)).unwrap();
    assert_eq!(report.completion, "done");
    assert_eq!(
        kinds(&report.lines),
        [
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "tool_called",
            "tool_result",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "handoff_written",
            "run_frozen",
        ]
    );

    let frozen: serde_json::Value = serde_json::from_slice(report.lines.last().unwrap()).unwrap();
    assert_eq!(frozen["data"]["completion"], "done");
    assert_eq!(frozen["data"]["evidence"][0]["kind"], "model_returned");
    citysim::check_chain(report.lines).unwrap();
}

#[test]
fn the_loop_is_byte_deterministic() {
    let one = run_scenario(scenario(None)).unwrap();
    let two = run_scenario(scenario(None)).unwrap();
    assert_eq!(one.lines, two.lines, "same scenario, same bytes");
}

#[test]
fn a9_cancel_before_assemble_stops_before_any_window_bytes() {
    let report = run_scenario(scenario(Some(CancelPoint::BeforeAssemble { turn: 1 }))).unwrap();
    assert_eq!(report.completion, "cancelled");
    let ks = kinds(&report.lines);

    assert_eq!(
        ks,
        [
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "tool_called",
            "tool_result",
            "cancel_received",
            "handoff_written",
            "run_frozen",
        ]
    );
    citysim::check_chain(report.lines).unwrap();
}

#[test]
fn a9_cancel_before_call_leaves_the_assembled_prompt_on_the_ledger() {
    let report = run_scenario(scenario(Some(CancelPoint::BeforeCall { turn: 0 }))).unwrap();
    assert_eq!(report.completion, "cancelled");
    assert_eq!(
        kinds(&report.lines),
        [
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "prompt_shape_compared",
            "cancel_received",
            "handoff_written",
            "run_frozen",
        ]
    );
}

#[test]
fn a9_cancel_before_wave_completes_the_model_call_but_runs_no_tool() {
    let report = run_scenario(scenario(Some(CancelPoint::BeforeWave { turn: 0 }))).unwrap();
    assert_eq!(report.completion, "cancelled");
    let ks = kinds(&report.lines);
    assert_eq!(
        ks,
        [
            "checkpoint_committed",
            "run_started",
            "prompt_assembled",
            "prompt_shape_compared",
            "model_called",
            "model_returned",
            "cancel_received",
            "handoff_written",
            "run_frozen",
        ]
    );
    assert!(
        !ks.contains(&"tool_called".to_owned()),
        "cancel at the wave boundary must run no tool"
    );
}

#[test]
fn a_run_takes_every_turn_its_work_asks_for_and_then_concludes() {
    let mut sc = scenario(None);
    sc.model = ScriptModel::new(
        (0..16)
            .map(|_| ModelReturn {
                usage: None,
                stop: None,
                billed_usd_micros: None,
                message: Payload::empty(),
                calls: vec![probe_call()],
            })
            .chain(std::iter::once(concluding("sixteen waves walked").unwrap()))
            .collect(),
    );
    sc.bench = bench_with(vec![Box::new(ScriptTool::new(
        probe_meta(),
        (0..16)
            .map(|_| {
                Ok(ToolOutcome {
                    result: Payload::empty(),
                    attachments: Vec::new(),
                })
            })
            .collect(),
    ))]);
    let report = run_scenario(sc).unwrap();
    assert_eq!(report.completion, "done");
    let ks = kinds(&report.lines);
    assert_eq!(
        ks.iter().filter(|k| *k == "prompt_shape_compared").count(),
        17
    );
    assert_eq!(ks.iter().filter(|k| *k == "prompt_assembled").count(), 1);
    assert_eq!(ks.last().unwrap(), "run_frozen");
}
