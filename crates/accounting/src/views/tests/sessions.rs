// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Place, view_record};
use crate::views::Views;
use kernel::{Address, EventKind, Payload, RunId};

/// A room's stretches come from the fold: where each began, how, its runs
/// and its last line, newest first, with nothing of another room in them,
/// answered without leaving the lock (wire-SPEC 8-71).
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
            serde_json::Map::new(),
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
                    stretch(1, wire::SessionStart::Dispatched, 1, 2),
                ],
                earlier: 0,
            }),
            wire::Answer::Sessions(wire::SessionsAnswer {
                room: there.clone(),
                sessions: vec![stretch(3, wire::SessionStart::Dispatched, 1, 7)],
                earlier: 0,
            }),
        ]
    );
}
