// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_cut_inside_a_wave_moves_back_to_the_safe_point() {
    let mother = mother();

    let at_call = mother.inherited(Seq::new(2)).unwrap();
    assert_eq!(at_call.at, Seq::new(0));
    assert!(
        at_call
            .messages
            .iter()
            .all(|message| message.role != Role::Assistant),
        "the half-answered wave is dropped whole: {:?}",
        at_call.messages
    );

    let at_open = mother.inherited(Seq::new(0)).unwrap();
    assert_eq!(at_open.at, Seq::new(0));
    assert_eq!(at_open.messages.len(), 1);
}

#[test]
fn a_cut_the_history_does_not_hold_is_refused() {
    let mother = mother();
    let err = mother.inherited(Seq::new(99)).unwrap_err();
    assert_eq!(err.code(), &kernel::AxCode::InvalidArgs);
    assert!(err.recovery().contains("ends at seq 3"), "{err}");
}

#[test]
fn an_old_call_is_rebuilt_as_it_was_recorded() {
    let older = Payload::of(&serde_json::json!({
        "path": "lab/notes.md", "old": "draft", "new": "final",
    }))
    .unwrap();
    let mut drafts = mother_drafts();
    drafts[1] = line(
        EventKind::ModelReturned,
        Payload::of(&ModelReturned {
            message: kernel::model::message_payload(&[ContentBlock::ToolUse {
                id: "tu_1".to_owned(),
                name: kernel::ToolName::parse("edit").unwrap(),
                input: older.clone(),
            }])
            .unwrap(),
            calls: 1,
            usage: None,
            stop: None,
            billed_usd_micros: None,
            first_at: None,
            first_us: None,
            took_us: None,
        })
        .unwrap(),
    );
    let inherited = verified(drafts).inherited(Seq::new(3)).unwrap();
    assert_eq!(
        inherited.messages[1].content,
        vec![ContentBlock::ToolUse {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("edit").unwrap(),
            input: older,
        }]
    );
}
