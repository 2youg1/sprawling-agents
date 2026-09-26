// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each building is working towards and who holds which part of its
//! plan: the part of the worker's state that `pursuit_changed` and the
//! `roadmap_*` records change (sprawling-SPEC.md 8-111).

use std::collections::BTreeMap;
use std::path::Path;

use kernel::event::record::RoadmapMoved;
use kernel::{Address, AxError, EventKind, NodeId, Payload};

use super::super::building_of;

/// The planning the worker holds, one value because one family of
/// records changes it and a pursuit reads all three parts together.
pub(in crate::assembly) struct Planning {
    /// What each building is working towards. Held by the worker
    /// because the worker is what acts on it; rebuilt from the records
    /// on open.
    pub(in crate::assembly) pursuits: BTreeMap<Address, kernel::Pursuit>,
    /// The depth-zero position, the one thing that can declare a
    /// pursuit, minted once when the worker opens.
    pub(in crate::assembly) delegator: kernel::Delegator,
    /// Which room holds each node of each building's plan.
    pub(in crate::assembly) holders: PlanHolders,
    /// The one door a landing replaces a building's plan through:
    /// `city::edit_against`, and in a test a writer that refuses, because
    /// a read-only file does not stop the rename over it where the
    /// directory is writable (sprawling-SPEC.md 8-42-8).
    pub(in crate::assembly) write_plan: fn(&Path, &[u8], &[u8]) -> Result<(), AxError>,
}

impl Planning {
    /// Shows one line the worker wrote to the part of planning that is
    /// folded from lines. The pursuits are changed where they are
    /// decided (`plans`), before their record is written.
    ///
    /// # Errors
    /// As [`PlanHolders::absorb`].
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.holders.absorb(kind, addr, data)
    }
}

/// Which room holds each node of each building's plan, and the one
/// definition of how a claim record changes that: the restart's fold and
/// the live worker both call [`PlanHolders::absorb`].
///
/// The room is the record's own `addr`, so nothing here derives what the
/// claiming run already wrote down.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::assembly) struct PlanHolders(BTreeMap<Address, BTreeMap<NodeId, String>>);

impl PlanHolders {
    /// A record that names no building is left out: the table holds
    /// known holders and guesses at none.
    ///
    /// # Errors
    /// Refuses a `roadmap_*` payload that does not read as a
    /// [`RoadmapMoved`]: a claim whose node cannot be read would
    /// otherwise leave a holder the history already released.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "five kinds move a claim; the rest of the event vocabulary does not"
    )]
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::RoadmapClaimed => {
                let node = data.read::<RoadmapMoved>()?.node;
                if let (Some(building), Some(room)) = (addr.and_then(building_of), addr) {
                    self.0
                        .entry(building)
                        .or_default()
                        .insert(node, room.as_str().to_owned());
                }
            }
            // A split is its parent's fate as much as a finish is: the
            // run that split the node holds nothing afterwards
            // (sprawling-SPEC.md 8-42-8).
            EventKind::RoadmapFinished
            | EventKind::RoadmapReleased
            | EventKind::RoadmapSplit
            | EventKind::RoadmapBlocked => {
                let node = data.read::<RoadmapMoved>()?.node;
                if let Some(building) = addr.and_then(building_of) {
                    self.0.entry(building).or_default().remove(&node);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Which room holds each node of one building's plan.
    pub(in crate::assembly) fn in_building(&self, building: &Address) -> BTreeMap<NodeId, String> {
        self.0.get(building).cloned().unwrap_or_default()
    }
}
