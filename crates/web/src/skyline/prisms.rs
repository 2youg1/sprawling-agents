// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prisms: what a city of buildings looks like.

use std::collections::BTreeSet;

use channels::{Address, BuildingProgress, Progress};

use crate::isometry::CITY_EXTENT;

/// A Building's place on the grid, and how tall it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prism {
    pub id: String,
    pub u: i32,
    pub v: i32,
    /// Storeys: the logarithmic order of the Building's Asset count, so a
    /// city with one huge Building is still readable.
    pub storeys: u32,
    /// How many of those storeys the plan has finished.
    ///
    /// The reason the silhouette carries data at all: a tower's height is
    /// what its plan took on and this is the part that is done, so a person
    /// reads progress off the skyline instead of off a number beside it.
    /// A building with no denominator has no finished part either - the
    /// same refusal to invent a ratio that `Progress` makes in the type.
    pub done: u32,
    pub active: bool,
    /// What the tower says about itself under its own footprint - the
    /// plan's own numbers. Annotated directly rather than through a
    /// legend, which would ask a reader to hold a mapping in their head
    /// while looking somewhere else.
    pub note: String,
}

/// Places a Building deterministically from its id.
///
/// A stable hash rather than insertion order: the same city state must
/// render the same picture every time, or a bitmap comparison is worthless
/// and a person loses the spatial memory the whole metaphor is for.
#[must_use]
pub fn place(id: &str, extent: u32) -> (i32, i32) {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let span = u64::from(extent.max(1));
    let u = hash.checked_rem(span).unwrap_or_default();
    let v = hash
        .checked_div(span)
        .and_then(|rest| rest.checked_rem(span))
        .unwrap_or_default();
    (
        i32::try_from(u).unwrap_or_default(),
        i32::try_from(v).unwrap_or_default(),
    )
}

/// Storeys from an Asset count: the logarithmic order, floor of log2 plus
/// one. Linear height would make one large Building dwarf the city into
/// invisibility.
#[must_use]
pub fn storeys(assets: u64) -> u32 {
    if assets == 0 {
        return 1;
    }
    let bits = u64::BITS.saturating_sub(assets.leading_zeros());
    bits.max(1)
}

/// Orders prisms for the painter: ground, then Residents, then Buildings by
/// `u + v` ascending, so nearer prisms are drawn over farther ones.
#[must_use]
pub fn painter_order(mut prisms: Vec<Prism>) -> Vec<Prism> {
    prisms.sort_by_key(|prism| (prism.u.saturating_add(prism.v), prism.u, prism.id.clone()));
    prisms
}

/// The three faces' tokens. Form stands up on lightness difference, not on
/// outlines - an outlined box reads as a diagram, a lit box reads as a
/// solid.
#[must_use]
pub fn face_tokens(active: bool, selected: bool) -> (&'static str, &'static str, &'static str) {
    let top = if selected {
        "G7"
    } else if active {
        "G6"
    } else {
        "G5"
    };
    (top, "G4", "G2")
}

/// The prisms of a city as the server described it.
///
/// Two sources, on purpose. Where a building stands and how tall it is
/// come from the plan it published, which changes when someone writes a
/// roadmap. Whether it is lit comes from the runs in flight, which the
/// event stream already folds. Asking the server again on every event
/// would turn a fold into a poll.
#[must_use]
pub fn prisms_of(
    buildings: &[BuildingProgress],
    busy: &BTreeSet<Address>,
    said: crate::lang::Lang,
) -> Vec<Prism> {
    let mut prisms: Vec<Prism> = buildings
        .iter()
        .map(|building| {
            let (u, v) = place(building.addr.as_str(), CITY_EXTENT);
            let storeys = storeys(scale_of(building.progress));
            Prism {
                id: building.addr.as_str().to_owned(),
                u,
                v,
                storeys,
                done: done_storeys(building.progress, storeys),
                active: busy.contains(&building.addr),
                // The words for a plan's progress come from the one
                // module that writes them, so the
                // label under a tower and the bar on a building's page
                // cannot drift apart.
                note: crate::progress::bar(
                    &building.progress,
                    false,
                    crate::progress::Subject::Plan,
                    said,
                )
                .label,
            }
        })
        .collect();
    // The hash spreads buildings over a twelve-by-twelve square; the view
    // shows the part that is occupied. Re-basing to the corner of that
    // part is a translation, so it keeps the placement deterministic and
    // keeps drawing and picking reading the same coordinates.
    let low_u = prisms.iter().map(|prism| prism.u).min().unwrap_or_default();
    let low_v = prisms.iter().map(|prism| prism.v).min().unwrap_or_default();
    for prism in &mut prisms {
        prism.u = prism.u.saturating_sub(low_u);
        prism.v = prism.v.saturating_sub(low_v);
    }
    prisms
}

/// A building's size is the work it has taken on, not the work it has
/// finished: a building that just finished everything does not shrink.
/// Without a denominator there is no size to read, so the building is one
/// storey — the same refusal to invent a number that `Progress` makes in
/// the type.
pub(crate) fn scale_of(progress: Progress) -> u64 {
    match progress {
        Progress::Planned(planned) => u64::from(planned.ratio().1),
        Progress::Unplanned(_) => 0,
    }
}

/// How many storeys of a tower are finished.
///
/// The plan's own ratio, carried onto the height the tower actually has.
/// Rounded down, and never the whole tower unless the plan is whole: a
/// building one row short of done should not look done from across the
/// city, which is the only distance this picture is read from.
pub(crate) fn done_storeys(progress: Progress, storeys: u32) -> u32 {
    let Progress::Planned(planned) = progress else {
        return 0;
    };
    let (done, total) = planned.ratio();
    if total == 0 {
        return 0;
    }
    if done >= total {
        return storeys;
    }
    storeys
        .checked_mul(done)
        .and_then(|scaled| scaled.checked_div(total))
        .unwrap_or(0)
        .min(storeys.saturating_sub(1))
}

/// The rows a building's plan could not state. Shown rather than dropped:
/// a plan quietly missing two lines is worse than no plan, because it
/// reads as complete.
#[must_use]
pub fn unreadable_rows(buildings: &[BuildingProgress]) -> Vec<String> {
    let mut rows = Vec::new();
    for building in buildings {
        for problem in &building.problems {
            rows.push(format!("{}: {problem}", building.addr.as_str()));
        }
    }
    rows
}
