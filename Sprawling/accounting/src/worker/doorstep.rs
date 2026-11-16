// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::Address;

use super::commanding::door::InHand;
use super::{Entrance, Knock};

pub(in crate::worker) struct Doorstep {

    pub(in crate::worker) entrance: Entrance,

    pub(in crate::worker) knocks: Vec<Knock>,

    deferred: BTreeMap<Address, Knock>,

    pub(in crate::worker) in_hand: Option<InHand>,
}

impl Doorstep {

    pub(in crate::worker) fn opened(entrance: Entrance) -> Doorstep {
        Doorstep {
            entrance,
            knocks: Vec::new(),
            deferred: BTreeMap::new(),
            in_hand: None,
        }
    }

    pub(in crate::worker) fn queue(&mut self, knock: Knock) {
        if !self.knocks.iter().any(|queued| queued.addr == knock.addr) {
            self.knocks.push(knock);
        }
    }

    pub(in crate::worker) fn defer(&mut self, knock: Knock) {
        self.deferred.entry(knock.addr.clone()).or_insert(knock);
    }

    pub(in crate::worker) fn vacated(&mut self, addr: &Address) {
        if let Some(knock) = self.deferred.remove(addr) {
            self.queue(knock);
        }
    }
}
