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
use crate::window::Opening;

/// The two openings are two situations, and the words differ.
/// A session nobody assigned a task to gets the person's own line,
/// because a conversational turn wrapped in field labels reads as a
/// form and is answered as one.
#[test]
fn a_session_with_a_person_opens_in_the_persons_own_words() {
    let mut assigned = Window::new();
    assigned.push_task_lines("close the loop", "one turn, then stop", Opening::FromJob);
    let ContentBlock::Text { text } = &assigned.messages()[0].content[0] else {
        panic!("the dispatch lines are text");
    };
    assert_eq!(text, "Task: close the loop\nGoal: one turn, then stop");

    let mut talking = Window::new();
    talking.push_task_lines("what do you make of this", "", Opening::WithPerson);
    let ContentBlock::Text { text } = &talking.messages()[0].content[0] else {
        panic!("the dispatch line is text");
    };
    assert_eq!(text, "what do you make of this");
}

/// The job file's text is the prefix's run segment, so nothing sends
/// the agent to fetch what it was already handed. Before this, the
/// opening line carried a `cas:` hash no tool in the city can resolve.
#[test]
fn no_opening_line_points_at_a_file_the_agent_already_has() {
    for (goal, opening) in [
        ("stop when it builds", Opening::FromJob),
        ("", Opening::WithPerson),
    ] {
        let mut window = Window::new();
        window.push_task_lines("do the thing", goal, opening);
        let ContentBlock::Text { text } = &window.messages()[0].content[0] else {
            panic!("the opening is text");
        };
        assert!(!text.contains("FULL READ"), "{opening:?} still points away");
        assert!(!text.contains("cas:"), "{opening:?} carries a content hash");
    }
}

#[test]
fn the_window_folds_steer_into_the_open_user_message() {
    let mut window = Window::new();
    window.push_tool_results(vec![ContentBlock::ToolResult {
        tool_use_id: "call-1".to_owned(),
        content: "{}".to_owned(),
        is_error: false,
        attachments: Vec::new(),
    }]);
    window.push_steer("user", "look again");
    assert_eq!(
        window.messages().len(),
        1,
        "steer rides the open user message"
    );
    assert_eq!(window.messages()[0].content.len(), 2);
    window.push_assistant(vec![ContentBlock::Text {
        text: "ok".to_owned(),
    }]);
    window.push_steer("@planner", "hurry");
    assert_eq!(
        window.messages().len(),
        3,
        "steer after assistant opens a new message"
    );
}
