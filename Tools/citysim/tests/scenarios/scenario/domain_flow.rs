// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn the_domain_door_bites_an_out_of_domain_write_inside_the_loop() {
    let mut sc = scenario(None);
    let write_meta = ToolMeta {
        name: ToolName::parse("edit_elsewhere").unwrap(),
        disclosure: "writes outside the room; exists to be refused".into(),
        params: Payload::empty(),
        effect: Effect::Write {
            domain: Address::parse("other/building/file.md").unwrap(),
        },
        cost_tier: CostTier::Free,
        timeout: None,
        render: RenderIntent::Generic,
        temporal: Temporal::Timeless,
    };
    sc.model = ScriptModel::new(vec![
        ModelReturn {
            usage: None,
            stop: None,
            billed_usd_micros: None,
            message: Payload::empty(),
            calls: vec![ToolCall {
                id: "call-2".to_owned(),
                name: ToolName::parse("edit_elsewhere").unwrap(),
                args: Payload::empty(),
            }],
        },
        concluding("the door answered").unwrap(),
    ]);
    sc.bench = bench_with(vec![Box::new(ScriptTool::new(
        write_meta,
        vec![Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })],
    ))]);
    let report = run_scenario(sc).unwrap();
    assert_eq!(report.completion, "done");
    let denial = report
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .find(|v| v["kind"] == "tool_result")
        .unwrap();
    assert_eq!(denial["data"]["error"]["code"], "E_OUTSIDE_WRITE_DOMAIN");
    let gate = &denial["data"]["error"]["gate"];
    assert!(gate["rule"].as_str().unwrap().contains("write domain"));
    assert!(!gate["alternative"].as_str().unwrap().is_empty());
    citysim::check_chain(report.lines).unwrap();
}

#[test]
fn a_tool_error_flows_back_as_a_tool_result_and_the_loop_continues() {
    let mut sc = scenario(None);
    sc.bench = bench_with(vec![Box::new(ScriptTool::new(probe_meta(), vec![]))]);

    let report = run_scenario(sc).unwrap();
    assert_eq!(report.completion, "done");
    let last_result = report
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .rfind(|v| v["kind"] == "tool_result")
        .unwrap();
    assert_eq!(last_result["data"]["error"]["code"], "E_TOOL_UNAVAILABLE");
}

#[test]
fn a_scenario_ledger_forks_into_a_byte_identical_prefix() {
    let report = run_scenario(scenario(None)).unwrap();
    let verified = runtime::replay::verify_lines(report.lines.clone()).unwrap();
    let at = kernel::Seq::new(4);
    let prefix = runtime::fork::prefix(&verified, at).unwrap();
    assert_eq!(prefix.len(), 5);
    assert_eq!(prefix.as_slice(), &report.lines[..5]);

    let over = kernel::Seq::new(9999);
    assert!(runtime::fork::prefix(&verified, over).is_err());
}
