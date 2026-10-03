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

use super::super::*;
use crate::conversation::{Letter, LetterKind, Opening, Speaker};

fn planner() -> Speaker {
    Speaker::Resident(Letter {
        from: "planner".to_owned(),
        run: None,
        kind: LetterKind::Steer,
        sender: None,
    })
}

/// A `steer_received` line written before the envelope carries only
/// `@<room>`, and still folds, as a steer letter (collab D16); a new
/// line reads back as the speaker that wrote it.
#[test]
fn a_recorded_source_reads_back_as_its_speaker() {
    let speaker = Speaker::Resident(Letter {
        from: "lab/room1".to_owned(),
        run: Some(kernel::RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000001").unwrap()),
        kind: LetterKind::Reply,
        sender: Some("running".to_owned()),
    });
    assert_eq!(
        [
            Speaker::from_recorded(&speaker.recorded()).unwrap(),
            Speaker::from_recorded("user").unwrap(),
            Speaker::from_recorded("city").unwrap(),
            Speaker::from_recorded("@lab/room1").unwrap(),
        ],
        [
            speaker,
            Speaker::Person,
            Speaker::City,
            Speaker::Resident(Letter {
                from: "lab/room1".to_owned(),
                run: None,
                kind: LetterKind::Steer,
                sender: None,
            }),
        ]
    );
    assert!(Speaker::from_recorded("lab/room1").is_err());
}

/// The two openings are two situations, and the words differ.
/// A session nobody assigned a task to gets the person's own line,
/// because a conversational turn wrapped in field labels reads as a
/// form and is answered as one.
#[test]
fn a_session_with_a_person_opens_in_the_persons_own_words() {
    let mut assigned = Conversation::new();
    assigned.push_task_lines("close the loop", "one turn, then stop", Opening::FromJob);
    let ContentBlock::Text { text } = &assigned.messages()[0].content[0] else {
        panic!("the dispatch lines are text");
    };
    // The task is the run segment of the prefix already; repeating it
    // here would carry the person's line twice in every request.
    assert_eq!(
        text,
        "The task is in JOB.md above.\nGoal: one turn, then stop"
    );

    let mut talking = Conversation::new();
    talking.push_task_lines("what do you make of this", "", Opening::WithPerson);
    let ContentBlock::Text { text } = &talking.messages()[0].content[0] else {
        panic!("the dispatch line is text");
    };
    assert_eq!(text, "what do you make of this");
}

/// The job file's text is the prefix's run segment, so nothing sends
/// the agent to fetch what it was already handed, and no opening line
/// carries a `cas:` hash no tool in the city can resolve.
#[test]
fn no_opening_line_points_at_a_file_the_agent_already_has() {
    for (goal, opening) in [
        ("stop when it builds", Opening::FromJob),
        ("", Opening::WithPerson),
    ] {
        let mut conversation = Conversation::new();
        conversation.push_task_lines("do the thing", goal, opening);
        let ContentBlock::Text { text } = &conversation.messages()[0].content[0] else {
            panic!("the opening is text");
        };
        assert!(!text.contains("FULL READ"), "{opening:?} still points away");
        assert!(!text.contains("cas:"), "{opening:?} carries a content hash");
    }
}

#[test]
fn the_window_folds_steer_into_the_open_user_message() {
    let mut conversation = Conversation::new();
    conversation.push_tool_results(vec![ContentBlock::ToolResult {
        tool_use_id: "call-1".to_owned(),
        content: "{}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    }]);
    conversation.push_steer(&Speaker::Person, "look again");
    assert_eq!(
        conversation.messages().len(),
        1,
        "steer rides the open user message"
    );
    assert_eq!(conversation.messages()[0].content.len(), 2);
    conversation.push_assistant(vec![ContentBlock::Text {
        text: "ok".to_owned(),
    }]);
    conversation.push_steer(&planner(), "hurry");
    assert_eq!(
        conversation.messages().len(),
        3,
        "steer after assistant opens a new message"
    );
}

/// A reply with no content pushes no assistant message, so the results
/// that follow meet a user message already on the wire. They still land
/// in the window, and a steer held since the send rides after them.
#[test]
fn tool_results_after_an_empty_reply_reach_the_window() {
    let mut conversation = Conversation::new();
    conversation.push_task_lines("find it", "found", Opening::FromJob);
    conversation.mark_sent();
    conversation.push_steer(&Speaker::Person, "narrow the search");
    conversation.push_assistant(Vec::new());
    let result = ContentBlock::ToolResult {
        tool_use_id: "call-1".to_owned(),
        content: "{}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    };
    conversation.push_tool_results(vec![result.clone()]);
    let blocks: Vec<&ContentBlock> = conversation
        .messages()
        .iter()
        .flat_map(|message| message.content.iter())
        .collect();
    assert_eq!(
        blocks[1..],
        [
            &result,
            &ContentBlock::Text {
                text: "user: narrow the search".to_owned()
            }
        ]
    );
}

/// Results that rewrote a sent message leave it open again: a steer
/// that arrives before the next assembly joins it after those results
/// instead of waiting one more wave.
#[test]
fn a_steer_after_results_on_a_sent_message_joins_them() {
    let mut conversation = Conversation::new();
    conversation.push_task_lines("find it", "found", Opening::FromJob);
    conversation.mark_sent();
    conversation.push_assistant(Vec::new());
    let result = ContentBlock::ToolResult {
        tool_use_id: "call-1".to_owned(),
        content: "{}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    };
    conversation.push_tool_results(vec![result.clone()]);
    conversation.push_steer(&Speaker::Person, "narrow the search");
    let tail: Vec<&ContentBlock> = conversation
        .messages()
        .iter()
        .flat_map(|message| message.content.iter())
        .skip(1)
        .collect();
    assert_eq!(
        tail,
        [
            &result,
            &ContentBlock::Text {
                text: "user: narrow the search".to_owned()
            }
        ]
    );
}
