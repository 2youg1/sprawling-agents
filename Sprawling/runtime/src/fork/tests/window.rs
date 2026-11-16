// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A steer recorded after the request was assembled reached the mother at
/// the end of the next tool results, and the branch rebuilds it there: the
/// task message she sent stays the bytes she sent.
#[test]
fn a_steer_after_assembly_is_inherited_after_the_results() {
    let mut drafts = mother_drafts();
    drafts.splice(
        1..1,
        [
            line(EventKind::PromptShapeCompared, Payload::empty()),
            line(
                EventKind::SteerReceived,
                Payload::of(&SteerReceived {
                    source: "user".to_owned(),
                    text: "in metres".to_owned(),
                })
                .unwrap(),
            ),
        ],
    );
    let inherited = verified(drafts).inherited(Seq::new(5)).unwrap();

    let mut expected = crate::conversation::Conversation::new();
    expected.push_task_lines(
        "measure the meter",
        "a number is written down",
        crate::conversation::Opening::WithPerson,
        "the User",
    );
    expected.push_assistant(vec![ContentBlock::Text {
        text: "reading it now".to_owned(),
    }]);
    expected.push_tool_results(vec![
        ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: "{\"said\":\"42\"}".to_owned(),
            is_error: false,
            attachments: Vec::new(),
        },
        ContentBlock::Text {
            text: "user: in metres".to_owned(),
        },
    ]);
    assert_eq!(inherited.messages, expected.messages().to_vec());
}

/// A line written before a result named its renderer is rebuilt as the
/// JSON its model was shown, and the same answer drawn by version 2 is
/// rebuilt as version 2 draws it: each line keeps the bytes its model
/// read, whichever build wrote it.
#[test]
fn each_result_line_is_rebuilt_by_the_renderer_it_names() {
    let read = serde_json::json!({ "path": "lab/a.md", "bytes": 3, "text": "hi\n" });
    let rebuilt = |rendered: Option<kernel::event::record::Rendering>| {
        let mut drafts = mother_drafts();
        drafts[3] = line(
            EventKind::ToolResult,
            Payload::of(&ToolResult {
                tool_use_id: "tu_1".to_owned(),
                name: kernel::ToolName::parse("read").unwrap(),
                answer: ToolAnswer::Answered {
                    result: Payload::of(&read).unwrap(),
                },
                took_us: None,
                rendered,
                shown_bytes: None,
                attachments: Vec::new(),
            })
            .unwrap(),
        );
        let inherited = verified(drafts).inherited(Seq::new(3)).unwrap();
        match &inherited.messages[2].content[0] {
            ContentBlock::ToolResult { content, .. } => content.clone(),
            other => panic!("expected the result, got {other:?}"),
        }
    };
    assert_eq!(
        rebuilt(None),
        r#"{"bytes":3,"path":"lab/a.md","text":"hi\n"}"#
    );
    assert_eq!(
        rebuilt(Some(kernel::event::record::Rendering::V2)),
        "lab/a.md lines 0-0 of 1\n0|hi\n"
    );
}

/// The rebuild is the mother's own window, folded through the same type
/// the live loop folds through.
#[test]
fn a_branch_inherits_the_mother_window_message_for_message() {
    let inherited = mother().inherited(Seq::new(3)).unwrap();
    assert_eq!(inherited.at, Seq::new(3));

    let mut expected = crate::conversation::Conversation::new();
    expected.push_task_lines(
        "measure the meter",
        "a number is written down",
        crate::conversation::Opening::WithPerson,
        "the User",
    );
    expected.push_assistant(vec![ContentBlock::Text {
        text: "reading it now".to_owned(),
    }]);
    expected.push_tool_results(vec![ContentBlock::ToolResult {
        tool_use_id: "tu_1".to_owned(),
        content: "{\"said\":\"42\"}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    }]);
    assert_eq!(inherited.messages, expected.messages().to_vec());
    assert_eq!(inherited.messages[0].role, Role::User);
    assert_eq!(inherited.messages[1].role, Role::Assistant);
    assert_eq!(inherited.messages[2].role, Role::User);
}

/// A branch of a branch opened with only its mother's own turns: the
/// conversation the mother had inherited from its own mother was folded
/// into the mother's window but never into a rebuild of it.
#[test]
fn a_branch_of_a_branch_keeps_the_grandmother_conversation() {
    let daughter = RunId::from_bytes([7; 16]);
    let of_daughter = |mut draft: EventDraft| {
        draft.run = daughter;
        draft
    };
    let mut lines = mother_drafts();
    lines.push(of_daughter(line(
        EventKind::RunForked,
        Payload::of(&kernel::event::record::RunForked {
            from: RunId::CITY,
            at_seq: Seq::new(3),
        })
        .unwrap(),
    )));
    lines.push(of_daughter(line(
        EventKind::RunStarted,
        Payload::of(&RunStarted {
            task: "write it down twice".to_owned(),
            goal: "two numbers".to_owned(),
            ..RunStarted::default()
        })
        .unwrap(),
    )));
    lines.push(of_daughter(line(
        EventKind::ModelReturned,
        Payload::of(&ModelReturned {
            message: kernel::model::message_payload(&[ContentBlock::Text {
                text: "written".to_owned(),
            }])
            .unwrap(),
            calls: 0,
            usage: None,
            stop: None,
            billed_usd_micros: None,
            first_at: None,
            first_us: None,
            took_us: None,
        })
        .unwrap(),
    )));
    let history = verified(lines);

    let grandmother = history.inherited(Seq::new(3)).unwrap();
    let mut expected = crate::conversation::Conversation::new();
    expected.push_inherited(&grandmother.messages);
    expected.push_task_lines(
        "write it down twice",
        "two numbers",
        crate::conversation::Opening::WithPerson,
        "the User",
    );
    expected.push_assistant(vec![ContentBlock::Text {
        text: "written".to_owned(),
    }]);
    let granddaughter = history.inherited(Seq::new(6)).unwrap();
    assert_eq!(
        (granddaughter.at, granddaughter.messages),
        (Seq::new(6), expected.messages().to_vec())
    );
}
