// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_branch_inherits_rules_only_from_read_results_that_reached_the_window() {
    let mut drafts = mother_drafts();
    drafts[3] = read_result("tu_1", &["lab/pkg"]);
    let inherited = verified(drafts).inherited(Seq::new(3)).unwrap();
    assert_eq!(
        inherited.delivered,
        [Address::parse("lab/pkg").unwrap()].into_iter().collect()
    );
    let shown = serde_json::to_string(&inherited.messages).unwrap();
    assert!(shown.contains("## lab/pkg/AGENTS.md"), "{shown}");
}

#[test]
fn an_unfinished_wave_contributes_neither_its_messages_nor_its_rule_rooms() {
    let mut drafts = mother_drafts();
    drafts[3] = read_result("tu_1", &["lab/first"]);
    drafts.push(returned(2));
    drafts.push(read_result("tu_2", &["lab/second"]));
    let inherited = verified(drafts).inherited(Seq::new(5)).unwrap();
    assert_eq!(inherited.at, Seq::new(3));
    assert_eq!(
        inherited.delivered,
        [Address::parse("lab/first").unwrap()].into_iter().collect()
    );
}

#[test]
fn a_shortened_read_result_does_not_deliver_its_rule_rooms() {
    let mut drafts = mother_drafts();
    drafts[3] = read_result_with_text(
        "tu_1",
        &["lab/shortened"],
        &"lorem ipsum dolor sit amet ".repeat(2_000),
    );
    let inherited = verified(drafts).inherited(Seq::new(3)).unwrap();
    assert!(inherited.delivered.is_empty());
}

#[test]
fn another_tools_rules_field_is_not_read_evidence() {
    let mut drafts = mother_drafts();
    let mut result = read_result("tu_1", &["lab/pkg"]);
    result.data = Payload::of(&ToolResult {
        tool_use_id: "tu_1".to_owned(),
        name: kernel::ToolName::parse("status").unwrap(),
        answer: ToolAnswer::Answered {
            result: Payload::of(&serde_json::json!({
                "rules": [{"room": "lab/pkg", "text": "not a read rule"}],
            }))
            .unwrap(),
        },
        took_us: None,
        rendered: Some(kernel::event::record::Rendering::V4),
        shown_bytes: None,
        attachments: Vec::new(),
    })
    .unwrap();
    drafts[3] = result;
    assert!(
        verified(drafts)
            .inherited(Seq::new(3))
            .unwrap()
            .delivered
            .is_empty()
    );
}

#[test]
fn a_malformed_rule_room_leaves_a_diagnostic_instead_of_vanishing() {
    let mut drafts = mother_drafts();
    drafts[3] = read_result("tu_1", &["../room"]);
    let inherited = verified(drafts).inherited(Seq::new(3)).unwrap();
    assert!(inherited.delivered.is_empty());
    assert_eq!(inherited.rule_warnings.len(), 1);
}

#[test]
fn a_grandchild_merges_the_rule_rooms_of_its_whole_lineage() {
    let mut drafts = mother_drafts();
    drafts[3] = read_result("tu_1", &["lab/grandmother"]);
    let daughter = RunId::from_bytes([7; 16]);
    let of_daughter = |mut draft: EventDraft| {
        draft.run = daughter;
        draft
    };
    drafts.push(of_daughter(line(
        EventKind::RunForked,
        Payload::of(&kernel::event::record::RunForked {
            from: RunId::CITY,
            at_seq: Seq::new(3),
        })
        .unwrap(),
    )));
    drafts.push(of_daughter(line(
        EventKind::RunStarted,
        Payload::of(&RunStarted {
            task: "continue".to_owned(),
            ..RunStarted::default()
        })
        .unwrap(),
    )));
    drafts.push(of_daughter(returned(1)));
    drafts.push(of_daughter(read_result("tu_2", &["lab/mother"])));
    let inherited = verified(drafts).inherited(Seq::new(7)).unwrap();
    assert_eq!(
        inherited.delivered,
        [
            Address::parse("lab/grandmother").unwrap(),
            Address::parse("lab/mother").unwrap(),
        ]
        .into_iter()
        .collect(),
        "{inherited:?}"
    );
}

/// A mother run with a line of a newer kind inside it: the rebuild skips
/// the line and reaches the conversation the same run holds without it,
/// rather than refusing it.
#[test]
fn a_mother_run_holding_a_line_of_a_newer_kind_rebuilds_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let drafts = mother_drafts();
    let (head, tail) = drafts.split_at(2);
    {
        let (mut ledger, _) = storage::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.append_all(head.to_vec()).unwrap();
    }
    let segment = storage::ledger_segments_at(dir.path())
        .unwrap()
        .pop()
        .unwrap();
    let mut written = std::fs::read(&segment).unwrap();
    let last = written
        .split(|byte| *byte == b'\n')
        .rfind(|line| !line.is_empty())
        .unwrap()
        .to_vec();
    let future = format!(
        "{{\"v\":1,\"run\":{},\"seq\":2,\"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\"data\":{{}},\"ig\":true}}\n",
        serde_json::to_string(&RunId::CITY).unwrap(),
        kernel::ledger::chain_hash(&last)
    );
    written.extend_from_slice(future.as_bytes());
    std::fs::write(&segment, written).unwrap();
    {
        let (mut ledger, _) = storage::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
        ledger.append_all(tail.to_vec()).unwrap();
    }
    let with_newer = Mother::written(dir).inherited(Seq::new(4)).unwrap();
    let without = mother().inherited(Seq::new(3)).unwrap();
    assert_eq!(
        with_newer,
        Inherited {
            at: Seq::new(4),
            ..without
        }
    );
}
