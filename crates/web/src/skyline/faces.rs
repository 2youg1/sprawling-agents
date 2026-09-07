// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The faces: prisms become geometry, and geometry becomes a list.

use crate::isometry::{CITY_EXTENT, Camera, Edge, Face, Label, Part, ground_of, storey_lift};

use super::prisms::{Prism, face_tokens, painter_order};

/// What to paint, in the order to paint it. A list of shapes rather than
/// a sequence of canvas calls: the browser turns it into calls, and a
/// headless run turns the same list into a bitmap, so the two cannot
/// drift apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayList {
    pub camera: Camera,
    /// The plate the city stands on, and its tiles. Its own field rather
    /// than the first faces, because everything in `faces` belongs to a
    /// building and is answerable to a click; the ground belongs to
    /// nobody.
    pub ground: Vec<Face>,
    pub faces: Vec<Face>,
    /// The outline of the selected building's top face, if one is picked.
    /// Stroked rather than filled, and the only stroke in the picture.
    pub outline: Option<[(i32, i32); 4]>,
    pub labels: Vec<Label>,
}

/// The three visible faces of one prism, top first.
///
/// This is the only place a prism becomes geometry. Drawing calls it and
/// so does picking, which is what makes "the pick follows the picture" a
/// property rather than a promise.
#[must_use]
pub fn faces_of(camera: &Camera, prism: &Prism, selected: bool) -> [Face; 3] {
    let (cx, cy) = camera.project(prism.u, prism.v);
    let half_width = i32::try_from(camera.tile_width.checked_div(2).unwrap_or(1)).unwrap_or(1);
    let half_height = i32::try_from(camera.tile_height.checked_div(2).unwrap_or(1)).unwrap_or(1);
    let lift = storey_lift(camera).saturating_mul(i32::try_from(prism.storeys).unwrap_or(1));

    // Ground diamond, then the same diamond lifted by the building's height.
    let north = (cx, cy.saturating_sub(half_height).saturating_sub(lift));
    let east = (cx.saturating_add(half_width), cy.saturating_sub(lift));
    let south = (cx, cy.saturating_add(half_height).saturating_sub(lift));
    let west = (cx.saturating_sub(half_width), cy.saturating_sub(lift));
    let ground_south = (south.0, south.1.saturating_add(lift));
    let ground_west = (west.0, west.1.saturating_add(lift));
    let ground_east = (east.0, east.1.saturating_add(lift));

    let (top, left, right) = face_tokens(prism.active, selected);
    [
        Face {
            id: prism.id.clone(),
            token: top,
            points: [north, east, south, west],
        },
        Face {
            id: prism.id.clone(),
            token: left,
            points: [west, south, ground_south, ground_west],
        },
        Face {
            id: prism.id.clone(),
            token: right,
            points: [south, east, ground_east, ground_south],
        },
    ]
}

/// Turns a city into the shapes that draw it.
///
/// The order is the painter's: farther prisms first, so nearer ones cover
/// them. Within a prism the top face comes first, because the sides hang
/// below it and nothing of the same prism can occlude it.
#[must_use]
pub fn draw(camera: &Camera, prisms: Vec<Prism>, selected: Option<&str>) -> DisplayList {
    let extent = occupied_extent(&prisms);
    let mut faces = Vec::new();
    let mut labels = Vec::new();
    let mut outline = None;
    for prism in painter_order(prisms) {
        let is_selected = selected.is_some_and(|id| id == prism.id);
        let sides = faces_of(camera, &prism, is_selected);
        if is_selected {
            outline = sides.first().map(|top| top.points);
        }
        faces.extend(sides);
        faces.extend(done_band_of(camera, &prism));
        faces.extend(windows_of(camera, &prism));
        labels.extend(labels_of(camera, &prism));
    }
    DisplayList {
        camera: *camera,
        ground: ground_of(camera, extent),
        faces,
        outline,
        labels,
    }
}

/// How wide a grid the placed buildings actually occupy.
///
/// The camera fits *this* rather than the hash's whole square: two
/// buildings in a twelve-by-twelve grid are two specks in an empty field,
/// and a city view whose subject is too small to read is a decoration.
#[must_use]
pub fn occupied_extent(prisms: &[Prism]) -> u32 {
    let span = |values: Vec<i32>| -> u32 {
        let low = values.iter().copied().min().unwrap_or_default();
        let high = values.iter().copied().max().unwrap_or_default();
        u32::try_from(high.saturating_sub(low).saturating_add(1)).unwrap_or(1)
    };
    let us = span(prisms.iter().map(|prism| prism.u).collect());
    let vs = span(prisms.iter().map(|prism| prism.v).collect());
    us.max(vs).saturating_add(1).clamp(3, CITY_EXTENT)
}

/// The lit band up a tower's two walls: the part of the plan that is done.
///
/// Drawn over the walls rather than instead of them, from the base up, so
/// the reading is the one a person already has for a filled bar - except
/// that here the bar is the building. A tower with nothing finished gets
/// no band at all, which is not the same as a band of zero height: one
/// says nothing is done, the other would be drawing a claim about a plan
/// that has no denominator.
#[must_use]
pub fn done_band_of(camera: &Camera, prism: &Prism) -> Vec<Face> {
    if prism.done == 0 {
        return Vec::new();
    }
    let unit = storey_lift(camera);
    let done = unit.saturating_mul(i32::try_from(prism.done).unwrap_or(0));
    let [_, left, right] = faces_of(camera, prism, false);
    let mut band = Vec::new();
    for wall in [left, right] {
        // A wall runs top-edge, top-edge, base, base. The finished part
        // rises `done` from the base.
        let (Some(top_a), Some(top_b), Some(base_b), Some(base_a)) = (
            wall.points.first().copied(),
            wall.points.get(1).copied(),
            wall.points.get(2).copied(),
            wall.points.get(3).copied(),
        ) else {
            continue;
        };
        let raise = |point: (i32, i32)| (point.0, point.1.saturating_sub(done));
        let (head_a, head_b) = (raise(base_a), raise(base_b));
        // Never above the roof, whatever rounding did.
        let clamp = |head: (i32, i32), roof: (i32, i32)| (head.0, head.1.max(roof.1));
        band.push(Face {
            id: prism.id.clone(),
            token: "G7",
            points: [clamp(head_a, top_a), clamp(head_b, top_b), base_b, base_a],
        });
    }
    band
}

/// The windows of one tower.
///
/// Unlit windows are drawn too: "lit" only means
/// something where there is an unlit one beside it. A building with work
/// in flight lights one window per storey, which is the only place
/// activity is said in colour rather than in lightness.
#[must_use]
pub fn windows_of(camera: &Camera, prism: &Prism) -> Vec<Face> {
    let unit = storey_lift(camera);
    if unit < 6 {
        // Below this a window is a smudge, and a smudge is noise.
        return Vec::new();
    }
    let [top, _, _] = faces_of(camera, prism, false);
    let (Some(west), Some(south), Some(east)) =
        (top.points.first(), top.points.get(2), top.points.get(1))
    else {
        return Vec::new();
    };
    // The two visible walls hang from the top diamond's near edges.
    let walls = [(*west, *south), (*south, *east)];
    let mut windows = Vec::new();
    for storey in 0..prism.storeys {
        let drop = unit.saturating_mul(i32::try_from(storey).unwrap_or_default());
        let head = drop.saturating_add(unit.checked_div(4).unwrap_or(1));
        let foot = drop.saturating_add(unit.saturating_mul(3).checked_div(4).unwrap_or(1));
        for (wall, (from, to)) in walls.iter().enumerate() {
            let edge = Edge {
                from: *from,
                to: *to,
            };
            for slot in 0i32..2 {
                let eighth = |offset: i32| Part {
                    num: slot.saturating_mul(4).saturating_add(offset),
                    den: 8,
                };
                let (near, far) = (eighth(1), eighth(3));
                let lit =
                    prism.active && storey.checked_rem(2) == Some(0) && wall == 0 && slot == 1;
                windows.push(Face {
                    id: prism.id.clone(),
                    token: if lit { "G9" } else { "G3" },
                    points: [
                        edge.at(near, head),
                        edge.at(far, head),
                        edge.at(far, foot),
                        edge.at(near, foot),
                    ],
                });
            }
        }
    }
    windows
}

/// A tower's two lines: its name, and what its own plan says about it.
#[must_use]
pub fn labels_of(camera: &Camera, prism: &Prism) -> Vec<Label> {
    let (cx, cy) = camera.project(prism.u, prism.v);
    let below = cy
        .saturating_add(i32::try_from(camera.tile_height).unwrap_or_default())
        .saturating_add(4);
    vec![
        Label {
            id: prism.id.clone(),
            at: (cx, below),
            text: prism.id.clone(),
            token: if prism.active { "G10" } else { "G8" },
            leading: true,
        },
        Label {
            id: prism.id.clone(),
            at: (cx, below.saturating_add(14)),
            text: prism.note.clone(),
            token: "G6",
            leading: false,
        },
    ]
}
