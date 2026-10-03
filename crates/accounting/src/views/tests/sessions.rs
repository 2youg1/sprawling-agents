// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Place, view_record};
use crate::views::Views;
use kernel::{Address, EventKind, Payload, RunId};

/// A room's stretches come from the fold: where each began, how, its runs
/// and its last line, newest first, with nothing of another room in them,
/// answered without leaving the lock (`crates/wire/Spec.lean` §8-71).
#[test]
fn a_room_s_sessions_are_answered_from_the_fold() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let (here, there) = (
        Address::parse("lab/room1").unwrap(),
        Address::parse("lab/room2").unwrap(),
    );
    let runs = [1u8, 2, 3].map(|n| RunId::from_bytes([n; 16]));
    let branch = kernel::Origin {
        run: runs[2],
        at_seq: kernel::Seq::new(5),
    };
    let opened = |carried, from| {
        Payload::of(&kernel::event::record::SessionOpened { carried, from })
            .unwrap()
            .as_map()
            .clone()
    };
    let lines = [
        (
            1,
            runs[0],
            EventKind::RunStarted,
            &here,
            serde_json::Map::new(),
        ),
        (
            2,
            runs[0],
            EventKind::RunFrozen,
            &here,
            serde_json::Map::new(),
        ),
        (
            3,
            runs[1],
            EventKind::RunStarted,
            &there,
            serde_json::json!({ "dispatched_by": "person" })
                .as_object()
                .unwrap()
                .clone(),
        ),
        (
            4,
            RunId::CITY,
            EventKind::SessionOpened,
            &here,
            opened(true, None),
        ),
        (
            5,
            runs[2],
            EventKind::RunStarted,
            &here,
            serde_json::Map::new(),
        ),
        (
            6,
            RunId::CITY,
            EventKind::SessionOpened,
            &here,
            opened(false, Some(branch)),
        ),
        (
            7,
            runs[1],
            EventKind::RunFrozen,
            &there,
            serde_json::Map::new(),
        ),
    ];
    for (seq, run, kind, addr, data) in lines {
        views
            .apply(&view_record(Place { seq, run }, kind, addr, data))
            .unwrap();
    }
    let stretch = |began, start, runs, last| wire::SessionLine {
        began: kernel::Seq::new(began),
        start,
        runs,
        last: kernel::Seq::new(last),
        at: kernel::TimeMs::new(1_000),
        name: None,
        model: None,
        effort: None,
        workspace: None,
        preview: None,
    };
    let asked = |room: &Address| wire::Query::Sessions { room: room.clone() };

    assert!(matches!(
        views.prepare(&asked(&here)),
        crate::views::prepared::Prepared::Held(_)
    ));
    assert_eq!(
        [&here, &there].map(|room| views.answer(&asked(room))),
        [
            wire::Answer::Sessions(wire::SessionsAnswer {
                room: here.clone(),
                sessions: vec![
                    stretch(
                        6,
                        wire::SessionStart::Opened {
                            carry: wire::Carry::Nothing,
                            from: Some(branch),
                        },
                        0,
                        6,
                    ),
                    stretch(
                        4,
                        wire::SessionStart::Opened {
                            carry: wire::Carry::Handoff,
                            from: None,
                        },
                        1,
                        5,
                    ),
                    stretch(1, wire::SessionStart::Dispatched { by: None }, 1, 2),
                ],
                earlier: 0,
            }),
            wire::Answer::Sessions(wire::SessionsAnswer {
                room: there.clone(),
                sessions: vec![stretch(
                    3,
                    wire::SessionStart::Dispatched {
                        by: Some(kernel::event::Who::Person),
                    },
                    1,
                    7,
                )],
                earlier: 0,
            }),
        ]
    );
}

/// A stretch carries the name its last `session_named` gave it, the model
/// and effort of its last run, and the start of its last reply cut to
/// `SESSION_PREVIEW_MAX` characters; an empty name takes the name back
/// (`crates/wire/spec/Answer/Sessions.lean` D27).
#[test]
fn a_session_line_carries_its_name_model_effort_and_last_reply() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([1; 16]);
    let map = |value: serde_json::Value| value.as_object().unwrap().clone();
    let long = "x".repeat(wire::SESSION_PREVIEW_MAX + 10);
    let lines = [
        (
            1,
            EventKind::RunStarted,
            map(serde_json::json!({ "effort": "high" })),
        ),
        (
            2,
            EventKind::ModelCalled,
            map(serde_json::json!({ "segments": [], "model": "m-1" })),
        ),
        (
            3,
            EventKind::ModelReturned,
            map(serde_json::json!({
                "message": { "role": "assistant", "content": [{ "kind": "text", "text": long }] },
                "calls": 0
            })),
        ),
        (
            4,
            EventKind::SessionNamed,
            map(serde_json::json!({ "began": 1, "name": "first" })),
        ),
        (
            5,
            EventKind::SessionNamed,
            map(serde_json::json!({ "began": 1, "name": "renamed" })),
        ),
    ];
    for (seq, kind, data) in lines {
        views
            .apply(&view_record(Place { seq, run }, kind, &room, data))
            .unwrap();
    }
    let answered = views.answer(&wire::Query::Sessions { room: room.clone() });
    let wire::Answer::Sessions(sessions) = answered else {
        panic!("a sessions question is answered with sessions: {answered:?}");
    };
    let line = &sessions.sessions[0];
    assert_eq!(
        (
            line.name.as_deref(),
            line.model.as_deref(),
            line.effort,
            line.preview.as_ref().map(|said| said.chars().count()),
        ),
        (
            Some("renamed"),
            Some("m-1"),
            Some(kernel::Effort::High),
            Some(wire::SESSION_PREVIEW_MAX)
        )
    );

    views
        .apply(&view_record(
            Place { seq: 6, run },
            EventKind::SessionNamed,
            &room,
            map(serde_json::json!({ "began": 1, "name": "" })),
        ))
        .unwrap();
    let wire::Answer::Sessions(sessions) = views.answer(&wire::Query::Sessions { room }) else {
        panic!("a sessions question is answered with sessions");
    };
    assert_eq!(sessions.sessions[0].name, None);
}
