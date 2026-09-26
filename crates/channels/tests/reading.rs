// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a turn's reader makes of a `provider_degraded` line.
//!
//! The kind has two writers: a provider refusal the carrier table sends
//! here, written as the error itself, and the vault's startup fallback to
//! session memory. Only the first changed what a turn did.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

fn degraded(data: Payload) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(1),
            who: "owner".to_owned(),
            addr: None,
            kind: EventKind::ProviderDegraded,
            data,
            ig: false,
        },
        Seq::FIRST,
        GENESIS_PREV,
    )
}

#[test]
fn a_vault_that_fell_back_to_session_memory_is_no_note_on_a_turn() {
    let notice = serde_json::json!({
        "component": "vault",
        "fallback": "session-memory",
        "persistence": "this-process",
        "reason": "platform service returned a different value",
    });
    let record = degraded(serde_json::from_value(notice).unwrap());

    assert_eq!(
        channels::note_of(EventKind::ProviderDegraded, &record),
        None
    );
}

#[test]
fn a_provider_refusal_is_still_a_refusal_on_the_turn() {
    let error = kernel::AxError::failure(kernel::AxCode::Provider, "call the model", "m-1")
        .with_recovery("try again once the provider answers");
    let record = degraded(Payload::of(&error).unwrap());

    assert_eq!(
        channels::note_of(EventKind::ProviderDegraded, &record),
        Some(channels::Note::Refused {
            error,
            at: Seq::FIRST
        })
    );
}
