// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use kernel::{Address, EventKind, EventRecord, Payload, RunId};

mod folding;
mod released;
mod released_ledger;
mod roadmaps;
mod sessions;

/// Where a test line sits in the ledger and which run wrote it.
pub(super) struct Place {
    pub(super) seq: u64,
    pub(super) run: RunId,
}

/// One record for a view test, carrying a payload and an address.
pub(super) fn view_record(
    Place { seq, run }: Place,
    kind: EventKind,
    addr: &Address,
    data: serde_json::Map<String, serde_json::Value>,
) -> EventRecord {
    EventRecord::from_draft(
        kernel::EventDraft {
            run,
            t: kernel::TimeMs::new(1_000),
            who: "lab/room1".to_owned(),
            addr: Some(addr.clone()),
            kind,
            data: Payload::new(data).unwrap(),
            ig: false,
        },
        kernel::Seq::new(seq),
        kernel::B3Hash::digest(b"prev"),
    )
}
