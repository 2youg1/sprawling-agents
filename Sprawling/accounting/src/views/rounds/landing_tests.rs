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

//! A `send` call as the rounds answer it: where its letter landed
//! (wire D42, accounting D48 (d)).

use wire::{EventDraft, EventKind, Payload, RunId, TimeMs};

/// One line of the sending session, as its ledger holds it.
fn sent_line(kind: EventKind, data: Payload) -> EventDraft {
    EventDraft {
        run: RunId::from_bytes([7u8; 16]),
        t: TimeMs::new(9),
        who: "lab/a".to_owned(),
        addr: None,
        kind,
        data,
        ig: false,
    }
}

fn json(data: serde_json::Value) -> Payload {
    Payload::new(data.as_object().unwrap().clone()).unwrap()
}

fn letter(id: &str, to: &str) -> collab::Signal {
    collab::Signal::new(
        kernel::event::record::SignalId::parse(id).unwrap(),
        kernel::event::record::SignalKind::Mention,
        "lab/a".to_owned(),
        kernel::Address::parse(to).unwrap(),
        kernel::Version::FIRST,
        Payload::empty(),
        TimeMs::new(9),
    )
    .unwrap()
}

/// Three `send` calls open at once in one turn; their letters are
/// enqueued out of call order. Each call carries the landing of its own
/// letter, paired by the room it names and then by `SignalId`, and a
/// letter with no `signal_landed` (an older Ledger) carries none
/// (wire D42).
#[test]
fn a_send_call_says_where_its_letter_landed() {
    use kernel::Ledger;
    use kernel::event::record::Landing;
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let run = RunId::from_bytes([7u8; 16]);
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    let send = |id: &str, to: &str| {
        sent_line(
            EventKind::ToolCalled,
            json(
                serde_json::json!({ "id": id, "name": "signal", "render": "signal",
                "args": { "action": "send", "to": to, "text": "the lexer" } }),
            ),
        )
    };
    let result = |id: &str| {
        sent_line(
            EventKind::ToolResult,
            json(serde_json::json!({ "tool_use_id": id, "name": "signal",
                                     "result": { "delivered": true } })),
        )
    };
    let landed = |id: &str, landing: &str| {
        sent_line(
            EventKind::SignalLanded,
            json(serde_json::json!({ "signal": id, "landing": landing })),
        )
    };
    let (to_c, to_b, to_d) = (
        letter("a-c1", "lab/c"),
        letter("a-b1", "lab/b"),
        letter("a-d1", "lab/d"),
    );
    let drafts = [
        sent_line(EventKind::ModelCalled, Payload::empty()),
        send("s1", "lab/c"),
        send("s2", "lab/b"),
        send("s3", "lab/d"),
        sent_line(EventKind::SignalEnqueued, to_b.enqueued_payload().unwrap()),
        sent_line(EventKind::SignalEnqueued, to_c.enqueued_payload().unwrap()),
        sent_line(EventKind::SignalEnqueued, to_d.enqueued_payload().unwrap()),
        result("s1"),
        result("s2"),
        result("s3"),
        landed("a-b1", "knocked"),
        landed("a-c1", "delivered"),
    ];
    for draft in drafts {
        ledger.append(draft).unwrap();
    }
    drop(ledger);
    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run }) else {
        panic!("Rounds answers with rounds");
    };
    assert_eq!(
        answer.turns[0]
            .calls
            .iter()
            .map(|call| call.landing)
            .collect::<Vec<_>>(),
        vec![Some(Landing::Delivered), Some(Landing::Knocked), None]
    );
}
