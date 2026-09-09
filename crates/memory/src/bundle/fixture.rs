// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one city the bundle tests export, restore and weigh.
//!
//! Three test modules used to carry a byte-identical copy of it, which
//! is three authorities on what a city looks like: changing one left the
//! other two asserting against a shape that no longer existed
//! (memory-SPEC.md 8-21).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use kernel::{EventDraft, EventKind, Ledger, Payload, RunId, TimeMs};

use super::manifest::{CAS, LEDGER, RESERVED};
use crate::jsonl::JsonlLedger;

/// A city with `records` lines in its history, two files a bundle has to
/// carry, and a content store. The chain is walked as it is written,
/// which is what proves the records are one history rather than a pile.
pub(crate) fn city_with(records: u64, root: &Path) {
    let dir = root.join(RESERVED).join(LEDGER);
    let (mut ledger, _report) = JsonlLedger::open(&dir, TimeMs::new(1)).unwrap();
    for step in 0..records {
        ledger
            .append(EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(step.saturating_add(1)),
                who: "owner".to_owned(),
                addr: None,
                kind: EventKind::CityInitialized,
                data: Payload::empty(),
                ig: false,
            })
            .unwrap();
    }
    std::fs::create_dir_all(root.join("lab")).unwrap();
    std::fs::write(root.join("City.md"), b"# City.md\n").unwrap();
    std::fs::write(root.join("lab").join("Roadmap.md"), b"# Roadmap\n").unwrap();
    let cas = crate::Cas::open(&root.join(RESERVED).join(CAS)).unwrap();
    drop(cas);
}
