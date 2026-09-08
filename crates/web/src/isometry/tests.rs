// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::skyline::{Prism, draw, place, storeys};

#[test]
fn the_window_holds_everything_that_was_drawn() {
    // What replaced "a fitted city is inside the viewport it was
    // fitted to". The window is now derived from the drawing rather
    // than the drawing fitted into a window, so the property is
    // stronger: it cannot fail for a city of any shape.
    let list = draw(&Camera::tiles(), city(), None);
    let frame = view_box(&list, 0, (0, 0));
    let right = frame
        .x
        .saturating_add(i32::try_from(frame.width).unwrap_or(i32::MAX));
    let bottom = frame
        .y
        .saturating_add(i32::try_from(frame.height).unwrap_or(i32::MAX));
    for face in list.ground.iter().chain(list.faces.iter()) {
        for (x, y) in face.points {
            assert!(
                x >= frame.x && x <= right && y >= frame.y && y <= bottom,
                "({x},{y}) escaped the window {}",
                frame.attr()
            );
        }
    }
}

#[test]
fn zooming_crops_rather_than_magnifying_and_the_last_stop_stops() {
    // With the window fitted to the drawing, scaling every tile scales
    // the window with it and the picture on screen does not move. So a
    // stop has to take a smaller window, and this is the assertion
    // that keeps somebody from "fixing" it back into a tile scale.
    let list = draw(&Camera::tiles(), city(), None);
    let whole = view_box(&list, 0, (0, 0));
    let closer = view_box(&list, 1, (0, 0));
    let closest = view_box(&list, 2, (0, 0));
    assert!(closer.width < whole.width && closer.height < whole.height);
    assert!(closest.width < closer.width);
    // Past the end it stops rather than wrapping round to the widest
    // view: a control that cannot go further should stop.
    assert_eq!(view_box(&list, 99, (0, 0)), closest);
}

#[test]
fn panning_moves_the_window_and_leaves_the_shapes_alone() {
    let list = draw(&Camera::tiles(), city(), None);
    let still = view_box(&list, 1, (0, 0));
    let moved = view_box(&list, 1, (PAN_STEP, -PAN_STEP));
    assert_eq!(moved.x, still.x + PAN_STEP);
    assert_eq!(moved.y, still.y - PAN_STEP);
    assert_eq!((moved.width, moved.height), (still.width, still.height));
    // The drawing itself does not know it was panned.
    assert_eq!(draw(&Camera::tiles(), city(), None), list);
}

#[test]
fn a_tile_is_twice_as_wide_as_it_is_tall() {
    // Two halvings happen between a tile and a point, so an odd width
    // breaks the projection. The constant is the only place this can
    // now go wrong.
    let camera = Camera::tiles();
    assert_eq!(camera.tile_height * 2, camera.tile_width);
    assert_eq!(TILE_WIDTH % 4, 0);
}

#[test]
fn one_shape_is_spelled_one_way() {
    assert_eq!(
        points_attr(&[(0, 1), (2, 3), (4, 5), (6, 7)]),
        "0,1 2,3 4,5 6,7"
    );
}

fn city() -> Vec<Prism> {
    ["lab", "vault", "mail"]
        .iter()
        .enumerate()
        .map(|(n, id)| {
            let (u, v) = place(id, 8);
            Prism {
                id: (*id).to_owned(),
                u,
                v,
                storeys: storeys(10u64.saturating_mul(u64::try_from(n).unwrap_or(0) + 1)),
                done: 0,
                active: n == 0,
                note: "3/7".to_owned(),
            }
        })
        .collect()
}
