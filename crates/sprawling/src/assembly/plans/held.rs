// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each building is working towards and who holds which part of its
//! plan: the part of the worker's state that `pursuit_changed` and the
//! `roadmap_*` records change (sprawling-SPEC.md 8-91).

use std::collections::BTreeMap;
use std::path::Path;

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
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) {
        self.holders.absorb(kind, addr, data);
    }
}

/// Which room holds each node of each building's plan, and the one
/// definition of how a claim record changes that: the restart's fold and
/// the live worker both call [`PlanHolders::absorb`].
///
/// The room is the record's own `addr`, so nothing here derives what the
/// claiming run already wrote down.
#[derive(Default)]
pub(in crate::assembly) struct PlanHolders(BTreeMap<Address, BTreeMap<NodeId, String>>);

impl PlanHolders {
    /// A record that names no building or no node is left out: the
    /// table holds known holders and guesses at none.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "four kinds move a claim; the rest of the event vocabulary does not"
    )]
    pub(in crate::assembly) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) {
        let (Some(room), Some(node)) = (addr, node_of(data)) else {
            return;
        };
        let Some(building) = building_of(room) else {
            return;
        };
        match kind {
            EventKind::RoadmapClaimed => {
                self.0
                    .entry(building)
                    .or_default()
                    .insert(node, room.as_str().to_owned());
            }
            EventKind::RoadmapFinished | EventKind::RoadmapReleased | EventKind::RoadmapBlocked => {
                self.0.entry(building).or_default().remove(&node);
            }
            _ => {}
        }
    }

    /// Which room holds each node of one building's plan.
    pub(in crate::assembly) fn in_building(&self, building: &Address) -> BTreeMap<NodeId, String> {
        self.0.get(building).cloned().unwrap_or_default()
    }
}

/// Which plan node a `roadmap_*` record names.
fn node_of(data: &Payload) -> Option<NodeId> {
    NodeId::parse(data.as_map().get("node")?.as_str()?).ok()
}
