// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What reached the city's door and has not yet become a run
//! (`crates/sprawling/Spec.lean` §8-111).

use std::collections::BTreeMap;

use kernel::Address;

use super::{Entrance, Knock};

/// The work that has arrived and is still outside: the commands this
/// city has answered by key, and the residents spoken to while nobody
/// was home.
pub(in crate::worker) struct Doorstep {
    /// Every command key this city has answered, and what it answered.
    /// Folded from the history, so a client retrying across a restart
    /// is still asking for one thing.
    pub(in crate::worker) entrance: Entrance,
    /// Residents who were spoken to while nobody was home. Held between
    /// the run that spoke and the runs that answer, because delivery
    /// happens after the speaker has frozen.
    pub(in crate::worker) knocks: Vec<Knock>,
    /// Residents spoken to while a run was working in their room, one
    /// knock per room, sent again when that run gives the room back.
    deferred: BTreeMap<Address, Knock>,
}

impl Doorstep {
    /// The doorstep of a worker that has just opened: the keys its
    /// history already answered, and nothing else yet.
    pub(in crate::worker) fn opened(entrance: Entrance) -> Doorstep {
        Doorstep {
            entrance,
            knocks: Vec::new(),
            deferred: BTreeMap::new(),
        }
    }

    /// Queues a knock, once per room: a resident spoken to twice before
    /// answering is woken once and reads both.
    pub(in crate::worker) fn queue(&mut self, knock: Knock) {
        if !self.knocks.iter().any(|queued| queued.addr == knock.addr) {
            self.knocks.push(knock);
        }
    }

    /// Holds a knock until the run working in its room has left.
    pub(in crate::worker) fn defer(&mut self, knock: Knock) {
        self.deferred.entry(knock.addr.clone()).or_insert(knock);
    }

    /// Sends the knock that waited for the room at `addr` to empty.
    pub(in crate::worker) fn vacated(&mut self, addr: &Address) {
        if let Some(knock) = self.deferred.remove(addr) {
            self.queue(knock);
        }
    }
}
