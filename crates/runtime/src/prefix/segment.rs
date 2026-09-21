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

use kernel::event::record::PromptSource;
use kernel::{Address, AxCode, AxError, B3Hash, SystemBlock};

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
    /// The four slots in prefix order, which is also the order the
    /// cache breakpoints are declared in on the wire.
    pub const ALL: [SegmentSlot; 4] = [
        SegmentSlot::City,
        SegmentSlot::Building,
        SegmentSlot::Resident,
        SegmentSlot::Run,
    ];

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

    /// Recomputes this segment's hash from the bytes a request will
    /// carry and checks it against the hash fixed at construction.
    ///
    /// # Errors
    /// The bytes and the recorded hash disagree — which is the one thing
    /// a frozen prefix may not survive.
    pub(crate) fn verified_hash(&self) -> Result<B3Hash, AxError> {
        let fresh = B3Hash::digest(&self.bytes);
        if fresh != self.hash {
            return Err(prefix_drifted(format!(
                "the {} segment's bytes hash to {fresh}, not the recorded {}",
                self.slot.as_str(),
                self.hash
            )));
        }
        Ok(fresh)
    }

    /// A segment whose stored hash does not describe its bytes, so the
    /// invariant check has something to refuse. Only a test can build
    /// one; production reaches the check through the bytes it sends.
    #[cfg(test)]
    pub(crate) fn mislabelled(slot: SegmentSlot, bytes: Vec<u8>, hash: B3Hash) -> FrozenSegment {
        FrozenSegment {
            slot,
            bytes,
            hash,
            sources: Vec::new(),
        }
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
    pub(crate) fn row(&self) -> PromptSource {
        PromptSource {
            addr: self.addr.clone(),
            kept: self.kept,
            marker: self.dropped > 0,
            dropped: self.dropped,
        }
    }
}

/// The refusal a prefix that no longer matches its own record earns.
///
/// A session cannot send one prefix and keep calling it frozen, and the
/// run's recorded history belongs to the bytes it was told; the two
/// ways out both leave this run's ledger alone.
fn prefix_drifted(subject: String) -> AxError {
    AxError::failure(AxCode::CasCorrupt, "send the frozen prefix", subject).with_recovery(
        "start a new session, or fork this run: the prefix this turn would send is not \
         the one the session froze, and the two cannot share a history",
    )
}

/// The same assertion over the copy of the segments a request carries:
/// four cache-marked system blocks, hashed and compared against the
/// hashes the run froze. The request is what the provider reads, so this
/// is the check that the wire form and the record agree.
///
/// # Errors
/// A request that does not carry exactly four marked blocks, or whose
/// blocks hash to something the run did not freeze.
pub fn verified_system_hashes(
    system: &[SystemBlock],
    frozen: &[B3Hash; 4],
) -> Result<[B3Hash; 4], AxError> {
    let [first, second, third, fourth] = system else {
        return Err(prefix_drifted(format!(
            "the request carries {} system blocks, not the four the prefix has",
            system.len()
        )));
    };
    let blocks = [first, second, third, fourth];
    let mut fresh: [B3Hash; 4] = [B3Hash::digest(b""); 4];
    for (index, (block, expected)) in blocks.iter().zip(frozen.iter()).enumerate() {
        let slot = SegmentSlot::ALL
            .get(index)
            .map_or("unknown", |slot| slot.as_str());
        if !block.cache {
            return Err(prefix_drifted(format!(
                "the {slot} segment lost its cache breakpoint"
            )));
        }
        let hash = B3Hash::digest(block.text.as_bytes());
        if hash != *expected {
            return Err(prefix_drifted(format!(
                "the {slot} segment hashes to {hash}, not the recorded {expected}"
            )));
        }
        if let Some(slot) = fresh.get_mut(index) {
            *slot = hash;
        }
    }
    Ok(fresh)
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
            serde_json::to_value(source.row()).unwrap(),
            serde_json::json!({ "addr": "City.md", "kept": 12, "marker": true, "dropped": 4 })
        );
    }

    /// A document that fitted whole says nothing was cut, rather than
    /// leaving a reader to infer it from a missing key.
    #[test]
    fn a_whole_document_reports_nothing_dropped_and_no_marker() {
        let source = SegmentSource::whole(Address::parse("City.md").unwrap(), 9);
        assert_eq!(
            serde_json::to_value(source.row()).unwrap(),
            serde_json::json!({ "addr": "City.md", "kept": 9, "marker": false, "dropped": 0 })
        );
    }
}
