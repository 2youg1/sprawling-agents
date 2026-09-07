// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The links: building of an address, and old live links.

use channels::Address;

use crate::app::Snapshot;
use crate::route::View;

/// The building a room is in. A room address is `building/room`, so the
/// building is everything before the last separator; an address with no
/// separator is already a building.
#[must_use]
pub fn building_of(addr: &Address) -> Option<Address> {
    let (building, _) = addr.as_str().rsplit_once('/')?;
    Address::parse(building).ok()
}

/// Which run an old `#/live/<uuid>` link means, once the client knows
/// which room it was in.
///
/// The router is pure and the room is a fact only the snapshot holds, so
/// the redirect happens here rather than in `route`. `None` means this
/// client has not folded that run's start yet, and the page says it is
/// still asking rather than that the link is broken.
#[must_use]
pub fn room_for_link(snapshot: &Snapshot, view: &View) -> Option<View> {
    let View::Run(run) = view else {
        return None;
    };
    snapshot.room_of(run).cloned().map(View::Session)
}
