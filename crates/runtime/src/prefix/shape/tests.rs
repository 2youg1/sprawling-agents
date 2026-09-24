// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one request's cache shape records, what a replay rebuilds from
//! those records, and the tail anchor that keeps a request's own body
//! inside the cached region.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{ContentBlock, Role};

use super::*;

fn message(role: Role, text: &str) -> ChatMessage {
    ChatMessage {
        role,
        cache: false,
        content: vec![ContentBlock::Text {
            text: text.to_owned(),
        }],
    }
}

fn tool_results(text: &str) -> ChatMessage {
    ChatMessage {
        role: Role::User,
        cache: false,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: text.to_owned(),
            is_error: false,
            attachments: Vec::new(),
        }],
    }
}

fn prefix() -> FrozenPrefix {
    FrozenPrefix::assemble(
        super::super::FrozenSegment::new(SegmentSlot::City, b"city".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Building, b"building".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Resident, b"resident".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Run, b"run".to_vec()),
    )
    .unwrap()
}

fn shape(messages: &[ChatMessage]) -> PromptShape {
    PromptShape::of(&prefix(), &[], messages).unwrap()
}

fn tool() -> ToolDef {
    ToolDef {
        name: kernel::ToolName::parse("exec").unwrap(),
        description: "run a command".to_owned(),
        input_schema: kernel::Payload::empty(),
    }
}

/// The two record payloads one request leaves: `prompt_assembled` for the
/// four segments, `prompt_shape_compared` for the rest - written through
/// the same constructors the run uses.
fn records(
    messages: &[ChatMessage],
    tools: &[ToolDef],
    baseline: Option<&PromptShape>,
) -> (serde_json::Value, serde_json::Value) {
    let prefix = prefix();
    let assembled = serde_json::to_value(prefix.prompt_payload().unwrap()).unwrap();
    let shape = PromptShape::of(&prefix, tools, messages).unwrap();
    let compared =
        serde_json::to_value(shape.recorded(shape.attribute(baseline)).unwrap()).unwrap();
    (assembled, compared)
}

/// Red: a run's first request has no baseline, and says so. A silent
/// `changed: false` would read as "nothing moved, expect a hit".
#[test]
fn a_missing_baseline_is_a_first_request_rather_than_nothing_moved() {
    let shape = shape(&[message(Role::User, "Task: dig")]);
    assert_eq!(shape.attribute(None), ShapeChanged::FirstRequest);
}

/// The acceptance: a miss names which of system, tools or run moved.
#[test]
fn a_moved_region_is_named_system_tools_or_run() {
    let first = shape(&[message(Role::User, "Task: dig")]);
    let grown = shape(&[
        message(Role::User, "Task: dig"),
        message(Role::Assistant, "reading"),
    ]);
    assert_eq!(
        grown.attribute(Some(&first)),
        ShapeChanged::Compared {
            system: Vec::new(),
            tools: PartChange::Same,
            run: PartChange::Moved,
        },
        "a longer conversation is the run region moving"
    );
    assert_eq!(
        first.attribute(Some(&first)),
        ShapeChanged::Compared {
            system: Vec::new(),
            tools: PartChange::Same,
            run: PartChange::Same,
        },
        "the same shape twice moves nothing"
    );
    let with_tool =
        PromptShape::of(&prefix(), &[tool()], &[message(Role::User, "Task: dig")]).unwrap();
    assert_eq!(
        with_tool.attribute(Some(&first)),
        ShapeChanged::Compared {
            system: Vec::new(),
            tools: PartChange::Moved,
            run: PartChange::Same,
        }
    );
}

/// Red: the tail anchor lands on the last message - the one carrying
/// the wave's tool results - so those results stay inside the cached
/// region instead of being paid for again on every request.
#[test]
fn the_tail_anchor_carries_the_trailing_tool_results_into_the_cache() {
    let mut messages = vec![
        message(Role::User, "Task: dig"),
        message(Role::Assistant, "running it"),
        tool_results("ok"),
    ];
    anchor_tail(&mut messages);
    assert!(
        messages[2].cache,
        "the tail anchor sits on the last message, tool results included"
    );
    assert!(
        !messages[0].cache && !messages[1].cache,
        "one anchor, at the tail"
    );
    assert!(
        messages[2]
            .content
            .iter()
            .any(|block| matches!(block, ContentBlock::ToolResult { .. })),
        "the anchored message is the one the trailing tool results ride in"
    );
}

/// The invariant the marker lives by: it changes no recorded byte,
/// so moving it can never move the history.
#[test]
fn the_breakpoint_marker_changes_no_recorded_byte() {
    let mut anchored = tool_results("ok");
    let plain = tool_results("ok");
    assert_eq!(
        serde_json::to_vec(&anchored).unwrap(),
        serde_json::to_vec(&plain).unwrap(),
        "the marker is outside serde"
    );
    anchored.cache = true;
    assert_eq!(
        serde_json::to_vec(&anchored).unwrap(),
        serde_json::to_vec(&plain).unwrap()
    );
    assert_eq!(
        shape(&[plain.clone()]),
        shape(&[anchored]),
        "and outside the shape a cache compares"
    );
}

#[test]
fn the_upper_bound_counts_segments_tool_schemas_and_the_conversation() {
    let messages = [message(Role::User, "Task: dig")];
    let bare = shape(&messages);
    let segments = ["city", "building", "resident", "run"]
        .iter()
        .map(|text| u64::try_from(text.len()).unwrap())
        .fold(0_u64, u64::saturating_add);
    let measured = segments
        + u64::try_from(serde_json::to_vec(&Vec::<ToolDef>::new()).unwrap().len()).unwrap()
        + u64::try_from(serde_json::to_vec(&messages).unwrap().len()).unwrap();
    assert_eq!(bare.request_upper_bound().unwrap(), measured);
    let with_tool = PromptShape::of(&prefix(), &[tool()], &messages).unwrap();
    assert!(
        with_tool.request_upper_bound().unwrap() > bare.request_upper_bound().unwrap(),
        "the tool schema bytes are part of what a request is billed for"
    );
}

#[test]
fn one_segment_moving_is_named_by_its_slot() {
    let first = shape(&[message(Role::User, "Task: dig")]);
    let moved = FrozenPrefix::assemble(
        super::super::FrozenSegment::new(SegmentSlot::City, b"city changed".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Building, b"building".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Resident, b"resident".to_vec()),
        super::super::FrozenSegment::new(SegmentSlot::Run, b"run".to_vec()),
    )
    .unwrap();
    let later = PromptShape::of(&moved, &[], &[message(Role::User, "Task: dig")]).unwrap();
    assert_eq!(
        later.attribute(Some(&first)),
        ShapeChanged::Compared {
            system: vec!["city".to_owned()],
            tools: PartChange::Same,
            run: PartChange::Same,
        }
    );
}

/// The acceptance: a replay rebuilds the same `PromptShape` out of the
/// two records a request left, and re-derives the attribution instead of
/// trusting the one written down.
#[test]
fn a_replay_rebuilds_the_same_shape_and_re_derives_the_attribution() {
    let first_messages = [message(Role::User, "Task: dig")];
    let later_messages = [message(Role::User, "Task: dig"), tool_results("ok")];
    let first = shape(&first_messages);
    let later = PromptShape::of(&prefix(), &[tool()], &later_messages).unwrap();

    let (first_assembled, first_compared) = records(&first_messages, &[], None);
    let (later_assembled, later_compared) = records(&later_messages, &[tool()], Some(&first));

    // The rebuild is the shape the run measured: same segments from
    // `prompt_assembled`, same tool and conversation parts from the
    // compared line.
    let recorded = PromptShapeCompared::deserialize(&later_compared).unwrap();
    let rebuilt = PromptShape::from_parts(
        segment_parts_of(&later_assembled).unwrap(),
        recorded.tools,
        recorded.run,
    );
    assert_eq!(rebuilt, later, "the records rebuild the same shape");

    // And the offline face agrees with the attribution both lines state.
    assert!(crate::replay::rebuild_shape(None, &first_assembled, &first_compared).is_ok());
    assert!(
        crate::replay::rebuild_shape(
            Some((&first_assembled, &first_compared)),
            &later_assembled,
            &later_compared
        )
        .is_ok()
    );
}

/// Fail-closed: a line whose attribution disagrees with the parts it
/// states is refused, not read as it was written.
#[test]
fn a_line_whose_attribution_disagrees_is_refused() {
    let first_messages = [message(Role::User, "Task: dig")];
    let later_messages = [message(Role::User, "Task: dig again")];
    let first = shape(&first_messages);
    let later = shape(&later_messages);
    let (first_assembled, first_compared) = records(&first_messages, &[], None);
    let (later_assembled, _) = records(&later_messages, &[], Some(&first));
    let lying = serde_json::to_value(later.recorded(ShapeChanged::FirstRequest).unwrap()).unwrap();
    assert!(
        crate::replay::rebuild_shape(
            Some((&first_assembled, &first_compared)),
            &later_assembled,
            &lying
        )
        .is_err(),
        "a recorded FirstRequest beside a comparable baseline is a lie"
    );
}
