// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use kernel::event::record::RoadmapMoved;
use kernel::{Address, AxError, EventKind, NodeId, Payload};

use super::super::building_of;

pub(in crate::worker) struct Planning {
    pub(in crate::worker) pursuits: BTreeMap<Address, kernel::Pursuit>,

    pub(in crate::worker) delegator: kernel::Delegator,

    pub(in crate::worker) holders: PlanHolders,

    pub(in crate::worker) write_plan: PlanWriter,
}

pub(in crate::worker) type PlanWriter = fn(&Path, &[u8], &[u8]) -> Result<(), AxError>;

impl Planning {
    pub(in crate::worker) fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        self.holders.absorb(kind, addr, data)
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::worker) struct PlanHolders(BTreeMap<Address, BTreeMap<NodeId, String>>);

impl PlanHolders {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "five kinds move a claim; the rest of the event vocabulary does not"
    )]
    pub(in crate::worker) fn absorb(
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

    pub(in crate::worker) fn in_building(&self, building: &Address) -> BTreeMap<NodeId, String> {
        self.0.get(building).cloned().unwrap_or_default()
    }
}
