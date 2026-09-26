// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A `watchdog_fired` line is the watchdog's record, not an error laid
//! flat: the reading of a turn reads it as that record, and a back-off
//! shows the provider failure it is waiting out.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use channels::{Note, note_of};
use kernel::{AxCode, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use serde_json::json;

fn record(kind: EventKind, data: serde_json::Value) -> EventRecord {
    let serde_json::Value::Object(map) = data else {
        panic!("a payload is an object")
    };
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([3; 16]),
            t: TimeMs::new(9),
            who: "builder@lab.1".to_owned(),
            addr: None,
            kind,
            data: Payload::new(map).unwrap(),
            ig: false,
        },
        Seq::FIRST,
        B3Hash::from_bytes([0; 32]),
    )
}

/// The bytes `runtime::watchdog` writes for a back-off, key for key.
#[test]
fn a_watchdog_back_off_reads_as_the_failure_it_waits_out() {
    let line = record(
        EventKind::WatchdogFired,
        json!({
            "action": "back_off",
            "until_ms": 5000,
            "code": AxCode::Provider.as_str(),
            "subject": "the provider answered 503",
            "corrections": 0,
            "provider_failures": 1,
        }),
    );
    let Some(Note::Refused { error, at }) = note_of(EventKind::WatchdogFired, &line) else {
        panic!(
            "a back-off is a refusal the run waits out: {:?}",
            note_of(EventKind::WatchdogFired, &line)
        );
    };
    assert_eq!(
        (error.code(), error.subject(), at),
        (&AxCode::Provider, "the provider answered 503", Seq::FIRST)
    );
}
