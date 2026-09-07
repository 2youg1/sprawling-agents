// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The same city draws the same shapes, through the production door.

use std::collections::BTreeSet;

use channels::{Address, BuildingProgress, PlannedProgress, Progress, UnplannedProgress};

use super::faces::{done_band_of, draw, faces_of};
use super::prisms::done_storeys;
use super::prisms::{
    Prism, face_tokens, painter_order, place, prisms_of, storeys, unreadable_rows,
};
use crate::isometry::Camera;

fn planned(addr: &str, done: u32, total: u32, problems: Vec<String>) -> BuildingProgress {
    BuildingProgress {
        blocked: Vec::new(),
        ready: 0,
        addr: Address::parse(addr).unwrap(),
        progress: Progress::Planned(PlannedProgress {
            done,
            blocked: 0,
            total,
            done_ppb: 0,
            blocked_ppb: 0,
        }),
        problems,
    }
}

#[test]
fn a_city_of_buildings_becomes_a_city_of_prisms() {
    let buildings = vec![
        planned("lab", 1, 8, Vec::new()),
        planned("mill", 0, 1, Vec::new()),
    ];
    let mut busy = BTreeSet::new();
    busy.insert(Address::parse("lab").unwrap());
    let prisms = prisms_of(&buildings, &busy, crate::lang::Lang::En);
    assert_eq!(prisms.len(), 2);
    let lab = prisms.iter().find(|p| p.id == "lab").unwrap();
    let mill = prisms.iter().find(|p| p.id == "mill").unwrap();
    assert!(lab.active, "a building with a run in it is lit");
    assert!(!mill.active);
    assert!(
        lab.storeys > mill.storeys,
        "height is the work taken on, and lab took on eight rows to mill's one"
    );
    assert_eq!(
        prisms_of(&buildings, &busy, crate::lang::Lang::En),
        prisms,
        "the same city places the same way twice"
    );
}

#[test]
fn a_building_without_a_denominator_gets_no_invented_height() {
    let buildings = vec![BuildingProgress {
        blocked: Vec::new(),
        ready: 0,
        addr: Address::parse("yard").unwrap(),
        progress: Progress::Unplanned(UnplannedProgress {
            steps: 40,
            budget: channels::BudgetUse::default(),
        }),
        problems: Vec::new(),
    }];
    let prisms = prisms_of(&buildings, &BTreeSet::new(), crate::lang::Lang::En);
    assert_eq!(
        prisms[0].storeys, 1,
        "forty steps is not a size; a plan is, and there is none"
    );
}

#[test]
fn a_plan_the_city_could_not_read_is_shown_rather_than_dropped() {
    let buildings = vec![planned(
        "lab",
        1,
        2,
        vec!["row 4 has three columns".to_owned()],
    )];
    let rows = unreadable_rows(&buildings);
    assert_eq!(rows, vec!["lab: row 4 has three columns".to_owned()]);
}

#[test]
fn the_lit_band_rises_with_the_plan_and_never_above_the_roof() {
    let camera = Camera::tiles();
    let tower = |done: u32| Prism {
        id: "lab".to_owned(),
        u: 0,
        v: 0,
        storeys: 4,
        done,
        active: false,
        note: String::new(),
    };
    let top_of = |prism: &Prism| {
        done_band_of(&camera, prism)
            .first()
            .map(|face| face.points[0].1)
    };
    let low = top_of(&tower(1)).unwrap();
    let high = top_of(&tower(3)).unwrap();
    assert!(high < low, "more done means the band reaches higher");
    let roof = faces_of(&camera, &tower(4), false)[0].points[0].1;
    assert!(
        top_of(&tower(4)).unwrap() >= roof,
        "a full band stops at the roof rather than growing past it"
    );
}

#[test]
fn a_building_with_nothing_done_gets_no_band_rather_than_an_empty_one() {
    // Not the same statement: a band of zero height claims a ratio,
    // and a building with no denominator has none to claim.
    let camera = Camera::tiles();
    let unplanned = Prism {
        id: "lab".to_owned(),
        u: 0,
        v: 0,
        storeys: 1,
        done: 0,
        active: false,
        note: String::new(),
    };
    assert!(done_band_of(&camera, &unplanned).is_empty());
    assert_eq!(
        done_storeys(
            Progress::Unplanned(UnplannedProgress {
                steps: 9,
                budget: channels::BudgetUse::default(),
            }),
            4
        ),
        0
    );
}

#[test]
fn a_plan_one_row_short_does_not_look_finished_from_across_the_city() {
    // The only distance this picture is read from.
    let planned = |done: u32, total: u32| planned("lab", done, total, Vec::new()).progress;
    assert_eq!(done_storeys(planned(6, 7), 4), 3, "short of done is short");
    assert_eq!(done_storeys(planned(7, 7), 4), 4, "and done is done");
}

#[test]
fn placement_is_a_function_of_the_id_and_nothing_else() {
    // The same city state renders the same picture, or a bitmap
    // comparison is worthless and spatial memory never forms.
    assert_eq!(place("acme/floor1", 16), place("acme/floor1", 16));
    assert_ne!(place("acme/floor1", 16), place("acme/floor2", 16));
    let (u, v) = place("anything", 16);
    assert!((0..16).contains(&u) && (0..16).contains(&v));
}

#[test]
fn height_is_logarithmic_so_one_giant_does_not_flatten_the_city() {
    assert_eq!(storeys(0), 1);
    assert_eq!(storeys(1), 1);
    assert_eq!(storeys(2), 2);
    assert_eq!(storeys(1024), 11);
    // A thousandfold difference in assets is an elevenfold difference in
    // height, not a thousandfold one.
    assert!(storeys(1_000_000) < storeys(1000) * 3);
}

#[test]
fn nearer_prisms_are_painted_last() {
    let prism = |id: &str, u: i32, v: i32| Prism {
        id: id.to_owned(),
        u,
        v,
        storeys: 1,
        done: 0,
        active: false,
        note: "1/2".to_owned(),
    };
    let ordered = painter_order(vec![
        prism("far", 0, 0),
        prism("near", 5, 5),
        prism("mid", 2, 1),
    ]);
    let names: Vec<&str> = ordered.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(names, ["far", "mid", "near"]);
}

#[test]
fn painter_order_is_total_so_two_renders_agree() {
    let prism = |id: &str, u: i32, v: i32| Prism {
        id: id.to_owned(),
        u,
        v,
        storeys: 1,
        done: 0,
        active: false,
        note: "1/2".to_owned(),
    };
    // Two prisms on the same depth line: the tiebreak must be total.
    let one = painter_order(vec![prism("b", 1, 2), prism("a", 1, 2)]);
    let other = painter_order(vec![prism("a", 1, 2), prism("b", 1, 2)]);
    assert_eq!(one, other);
}

#[test]
fn faces_differ_in_lightness_so_form_stands_without_outlines() {
    let (top, left, right) = face_tokens(false, false);
    let faces: std::collections::BTreeSet<&str> = [top, left, right].into_iter().collect();
    assert_eq!(faces.len(), 3, "three faces, three lightnesses");
    assert_ne!(face_tokens(true, false).0, face_tokens(false, false).0);
    assert_ne!(face_tokens(false, true).0, face_tokens(true, false).0);
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

#[test]
fn the_same_city_draws_the_same_shapes_in_the_same_order() {
    let camera = Camera::tiles();
    let first = draw(&camera, city(), None);
    let mut shuffled = city();
    shuffled.reverse();
    let second = draw(&camera, shuffled, None);
    assert_eq!(
        first, second,
        "the picture is a function of the city, not of the order the buildings arrived"
    );
    let sides = first
        .faces
        .iter()
        .filter(|face| face.token != "G3" && face.token != "G9")
        .count();
    assert_eq!(sides, 9, "three prisms, three faces each");
    assert!(
        first.labels.len() >= 6,
        "every tower says its name and what its plan says"
    );
}

#[test]
fn a_taller_building_stands_higher_and_a_selected_one_is_lighter() {
    let camera = Camera::tiles();
    let short = Prism {
        id: "a".to_owned(),
        u: 1,
        v: 1,
        storeys: 1,
        done: 0,
        active: false,
        note: "1/2".to_owned(),
    };
    let tall = Prism {
        storeys: 5,
        ..short.clone()
    };
    let short_top = faces_of(&camera, &short, false)[0].points[0].1;
    let tall_top = faces_of(&camera, &tall, false)[0].points[0].1;
    assert!(tall_top < short_top, "more storeys reach further up");

    let plain = faces_of(&camera, &short, false)[0].token;
    let picked = faces_of(&camera, &short, true)[0].token;
    assert_ne!(plain, picked, "selection is visible without colour");
}
