// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The navigation: everything the palette can reach.

use crate::app::Snapshot;
use crate::asking::watchable;
use crate::phase::Phase;
use crate::route::View;
use crate::route::destinations;
use channels::Address;

/// Everything the palette can reach, in the order a reader expects it.
///
/// Pages first because they are the answer most of the time, then the
/// buildings this city holds, then the sessions it knows of. Assembled
/// here because this is where the nav, the city answer and the run list
/// already meet; the palette holding its own list would be a second
/// answer to "where can a person go".
#[must_use]
pub(crate) fn reachable(
    snapshot: &Snapshot,
    city: Option<&channels::CityAnswer>,
    lang: crate::lang::Lang,
) -> Vec<crate::palette::Offer> {
    let mut offers: Vec<crate::palette::Offer> = destinations(snapshot)
        .into_iter()
        .map(|spot| crate::palette::Offer {
            label: crate::lang::say(lang, spot.label).to_owned(),
            kind: crate::palette::Kind::Page,
            going: spot.view,
        })
        .collect();
    if let Some(answer) = city {
        offers.extend(answer.buildings.iter().filter_map(|building| {
            let addr = Address::parse(building.addr.as_str()).ok()?;
            Some(crate::palette::Offer {
                label: building.addr.as_str().to_owned(),
                kind: crate::palette::Kind::Building,
                going: View::Building(addr),
            })
        }));
    }
    offers.extend(
        watchable(snapshot)
            .into_iter()
            .map(|(id, said)| crate::palette::Offer {
                label: said,
                kind: crate::palette::Kind::Session,
                going: View::Run(id),
            }),
    );
    offers
}

/// Which buildings have a run in flight, folded from the snapshot rather
/// than asked of the server: the event stream already says it, and a
/// second question would be a second answer.
#[must_use]
pub(crate) fn busy_buildings(snapshot: &Snapshot) -> std::collections::BTreeSet<Address> {
    snapshot
        .runs()
        .filter(|(_, row)| matches!(row.phase, Phase::Running | Phase::Waiting))
        .filter_map(|(_, row)| row.addr.clone())
        .filter_map(|addr| building_of(&addr))
        .collect()
}

/// The building an address belongs to: its first segment. The city keeps
/// the authority on that (a building is a top-level address); this is the
/// same rule read on the page, so a run in `lab/room1` lights `lab`.
pub(crate) fn building_of(addr: &Address) -> Option<Address> {
    let head = addr.as_str().split('/').next()?;
    Address::parse(head).ok()
}
