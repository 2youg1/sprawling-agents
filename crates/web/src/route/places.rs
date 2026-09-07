// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The places: nav entries, and what is showing.

use channels::Address;

use crate::app::Snapshot;

use super::view::{Lens, View};

/// One entry of the left nav.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub view: View,
    /// What this destination is called, in whichever language the
    /// person reads. The word itself lives in `web::lang`, so the nav
    /// and the translation cannot disagree about which page is which.
    pub label: crate::lang::Msg,
    /// How many things are waiting behind this destination, when waiting
    /// is a thing that can happen there.
    pub waiting: Option<u32>,
}

/// Every destination the left nav offers, in reading order.
///
/// One producer for the list, its wording and its badge: a destination
/// added here appears in the nav, in the router and in the test that
/// walks them, and cannot appear in two of the three.
///
/// **Five, and flat.** The previous nav had nine entries under three
/// headings, and the headings existed because nine entries read as a
/// menu to be searched rather than a place to go. Five is inside the
/// span a person holds without searching, so the headings are not
/// replaced by better headings — they are not needed.
#[must_use]
pub fn destinations(snapshot: &Snapshot) -> Vec<Destination> {
    let waiting = snapshot.waiting_on_you();
    vec![
        Destination {
            view: View::Sessions,
            label: crate::lang::Msg::NavSessions,
            waiting: None,
        },
        // The one badge in this interface. It is here on every page
        // because an unfinished thing that is out of sight stops being
        // an unfinished thing and starts being a surprise.
        Destination {
            view: View::Waiting,
            label: crate::lang::Msg::NavWaiting,
            waiting: (waiting > 0).then_some(waiting),
        },
        Destination {
            view: View::Record(Lens::Ledger),
            label: crate::lang::Msg::NavTheRecord,
            waiting: None,
        },
        Destination {
            view: View::Cost,
            label: crate::lang::Msg::NavCost,
            waiting: None,
        },
        Destination {
            view: View::Setup,
            label: crate::lang::Msg::NavSettings,
            waiting: None,
        },
    ]
}

/// Whether this destination is the page being shown.
///
/// The record's three lenses are one destination, so the nav entry stays
/// marked while a person moves between them: an entry that unhighlights
/// when the reader is still inside it says they have left.
#[must_use]
pub fn showing(destination: &View, view: &View) -> bool {
    match (destination, view) {
        (View::Record(_), View::Record(_)) => true,
        // A session and a building are reached from the list, and the
        // list stays lit while a person is inside one: they went deeper
        // into what the first entry offers rather than somewhere else.
        (View::Sessions, View::Session(_) | View::Building(_) | View::Run(_)) => true,
        (left, right) => left == right,
    }
}

/// The building a person is looking at, if the city page has one
/// selected. The nav does not carry buildings - a city may have fifty -
/// so the way in is the city page, and this is what it hands over.
#[must_use]
pub fn opened_building(selected: Option<&str>) -> Option<Address> {
    selected.and_then(|name| Address::parse(name).ok())
}

/// Where the `g` sequence's second key goes.
///
/// Here rather than in `web::keys` because a `View` carries a run id and
/// an address, and a module that decides what a key means has no business
/// holding either.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        dead_code,
        reason = "the only caller is the browser's keydown listener"
    )
)]
pub(crate) fn place_view(place: crate::keys::Place) -> View {
    match place {
        crate::keys::Place::Sessions => View::Sessions,
        crate::keys::Place::Waiting => View::Waiting,
        crate::keys::Place::Record => View::Record(Lens::Ledger),
        crate::keys::Place::Cost => View::Cost,
        crate::keys::Place::Setup => View::Setup,
    }
}
