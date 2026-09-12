// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One segment: its slot, its bytes, the documents behind it, and the
//! one row shape those documents are recorded in.
//!
//! **Why the row has a single author.** `prompt_assembled` is what an
//! offline rebuild reads to reconstruct a prefix (A15), and it is also
//! what a page reads to show a person which files their agent was given.
//! Two producers of that row — one for a prefix built from a plan, one
//! for a prefix assembled by hand at the city's own desk — would be two
//! spellings of the same fact, and `replay::rebuild_prefix` would only
//! understand whichever it was written against.

use kernel::{Address, B3Hash};
use serde_json::{Value, json};

/// The four slots in stability order; the order is the cache economics.
/// Exactly four — deliberately exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentSlot {
    City,
    Building,
    Resident,
    Run,
}

impl SegmentSlot {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SegmentSlot::City => "city",
            SegmentSlot::Building => "building",
            SegmentSlot::Resident => "resident",
            SegmentSlot::Run => "run",
        }
    }
}

/// One frozen segment: static bytes from frozen sources, hashed at
/// construction. The sole constructor takes bytes, not values — there is
/// deliberately no `From<TimeMs>` or any other volatile conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenSegment {
    slot: SegmentSlot,
    bytes: Vec<u8>,
    hash: B3Hash,
    sources: Vec<SegmentSource>,
}

impl FrozenSegment {
    /// A segment whose bytes name no documents - the resident slot is
    /// built from an identity and a catalog rather than from files.
    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment {
        FrozenSegment::assembled(slot, bytes, Vec::new())
    }

    /// A segment and the documents it was assembled from, which is what
    /// `prompt_assembled` records and what a page shows a person.
    pub fn assembled(
        slot: SegmentSlot,
        bytes: Vec<u8>,
        sources: Vec<SegmentSource>,
    ) -> FrozenSegment {
        let hash = B3Hash::digest(&bytes);
        FrozenSegment {
            slot,
            bytes,
            hash,
            sources,
        }
    }

    pub fn sources(&self) -> &[SegmentSource] {
        &self.sources
    }

    pub fn slot(&self) -> SegmentSlot {
        self.slot
    }

    pub fn hash(&self) -> &B3Hash {
        &self.hash
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One document a segment was assembled from, as the assembler saw it.
///
/// `kept` is how many bytes of that document reached the segment and
/// `dropped` is what the slot's budget cut off the end of it, so the
/// pair states both what the model read and what it did not. A document
/// that fitted whole reports `dropped: 0`, which is a measurement
/// rather than an absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentSource {
    pub addr: Address,
    pub kept: u64,
    pub dropped: u64,
}

impl SegmentSource {
    /// A document that reached its segment whole.
    pub fn whole(addr: Address, kept: u64) -> SegmentSource {
        SegmentSource {
            addr,
            kept,
            dropped: 0,
        }
    }

    /// The `prompt_assembled` row for this document.
    ///
    /// `marker` is carried because the rebuild reads it: it says a
    /// truncation marker follows the kept bytes, and the concatenation
    /// rule cannot be recovered from the byte counts alone.
    pub(crate) fn row(&self) -> Value {
        json!({
            "addr": self.addr.as_str(),
            "kept": self.kept,
            "marker": self.dropped > 0,
            "dropped": self.dropped,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    /// The four keys `replay::rebuild_prefix` reads, by the names it
    /// reads them under.
    #[test]
    fn a_row_names_the_four_keys_a_rebuild_reads() {
        let source = SegmentSource {
            addr: Address::parse("City.md").unwrap(),
            kept: 12,
            dropped: 4,
        };
        assert_eq!(
            source.row(),
            json!({ "addr": "City.md", "kept": 12, "marker": true, "dropped": 4 })
        );
    }

    /// A document that fitted whole says nothing was cut, rather than
    /// leaving a reader to infer it from a missing key.
    #[test]
    fn a_whole_document_reports_nothing_dropped_and_no_marker() {
        let source = SegmentSource::whole(Address::parse("City.md").unwrap(), 9);
        assert_eq!(
            source.row(),
            json!({ "addr": "City.md", "kept": 9, "marker": false, "dropped": 0 })
        );
    }
}
