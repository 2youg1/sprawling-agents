// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::PromptSource;
use kernel::{Address, AxCode, AxError, B3Hash, SystemBlock};

use super::BreakpointPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentSlot {
    City,
    Building,
    Resident,
    Run,
}

impl SegmentSlot {

    pub const ALL: [SegmentSlot; 4] = [
        SegmentSlot::City,
        SegmentSlot::Building,
        SegmentSlot::Resident,
        SegmentSlot::Run,
    ];

    pub fn shared_by_building(self) -> bool {
        match self {
            SegmentSlot::City | SegmentSlot::Building => true,
            SegmentSlot::Resident | SegmentSlot::Run => false,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SegmentSlot::City => "city",
            SegmentSlot::Building => "building",
            SegmentSlot::Resident => "resident",
            SegmentSlot::Run => "run",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenSegment {
    slot: SegmentSlot,
    bytes: Vec<u8>,
    hash: B3Hash,
    sources: Vec<SegmentSource>,
}

impl FrozenSegment {

    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment {
        FrozenSegment::assembled(slot, bytes, Vec::new())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentSource {
    pub addr: Address,
    pub kept: u64,
    pub dropped: u64,
    pub hash: B3Hash,
}

impl SegmentSource {

    pub fn whole(addr: Address, bytes: &[u8]) -> SegmentSource {
        SegmentSource::cut(addr, bytes, 0)
    }

    pub fn cut(addr: Address, kept: &[u8], dropped: u64) -> SegmentSource {
        SegmentSource {
            addr,
            kept: u64::try_from(kept.len()).unwrap_or(u64::MAX),
            dropped,
            hash: B3Hash::digest(kept),
        }
    }

    pub(crate) fn row(&self) -> PromptSource {
        PromptSource {
            addr: self.addr.clone(),
            kept: self.kept,
            marker: self.dropped > 0,
            dropped: self.dropped,
            hash: Some(self.hash),
        }
    }
}

pub(crate) const ANOTHER_ADDRESS: &str =
    "send this task to another address, which opens a session of its own";

fn prefix_drifted(subject: String) -> AxError {
    AxError::failure(AxCode::CasCorrupt, "send the frozen prefix", subject).with_recovery(format!(
        "{ANOTHER_ADDRESS}: the prefix this turn would send is not the one the session \
         froze, and the two cannot share a history"
    ))
}

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
        let marked = SegmentSlot::ALL
            .get(index)
            .is_some_and(|slot| BreakpointPlan::marks_edge(*slot));
        let slot = SegmentSlot::ALL
            .get(index)
            .map_or("unknown", |slot| slot.as_str());
        if block.cache && !marked {
            return Err(prefix_drifted(format!(
                "the {slot} segment carries a cache breakpoint the plan does not put there"
            )));
        }
        if !block.cache && marked {
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

    #[test]
    fn a_row_names_the_keys_a_rebuild_reads() {
        let source = SegmentSource::cut(Address::parse("City.md").unwrap(), b"city rules: ", 4);
        assert_eq!(
            serde_json::to_value(source.row()).unwrap(),
            serde_json::json!({
                "addr": "City.md",
                "kept": 12,
                "marker": true,
                "dropped": 4,
                "hash": B3Hash::digest(b"city rules: ").to_string(),
            })
        );
    }

    #[test]
    fn a_whole_document_reports_nothing_dropped_and_no_marker() {
        let source = SegmentSource::whole(Address::parse("City.md").unwrap(), b"the city.");
        assert_eq!(
            serde_json::to_value(source.row()).unwrap(),
            serde_json::json!({
                "addr": "City.md",
                "kept": 9,
                "marker": false,
                "dropped": 0,
                "hash": B3Hash::digest(b"the city.").to_string(),
            })
        );
    }
}
