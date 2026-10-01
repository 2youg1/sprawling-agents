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

use std::path::Path;

use super::*;
use crate::worker::fixture::*;
use crate::worker::*;

/// sprawling-SPEC.md 8-145: the lane that drove a run in a room under
/// review puts the stock back before the run comes home. `fly` is the
/// whole of what a lane runs and `land` is what the accounting thread
/// runs once the run is home, so a stock that stands ready when `fly`
/// returns, with nothing landed, is a stock no accounting thread made.
#[test]
fn a_lane_puts_the_stock_back_before_its_run_comes_home() {
    let dir = tempfile::tempdir().unwrap();
    lay_review_city(dir.path(), &["room1"]);
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let flown = fly_here(&mut worker, "lab/room1");
    let Flown::Model { site, driven, .. } = &flown else {
        panic!("a model dispatch flies a model run");
    };
    assert_eq!(
        (
            driven.is_ok(),
            site.lease.is_some(),
            stock_is_ready(dir.path())
        ),
        (true, true, true),
        "the run drove in a tree it borrowed, and its lane put the stock back"
    );
}

/// sprawling-SPEC.md 8-145: the stock a lane put back is what the next
/// room's first placement takes over, so that placement creates,
/// rewrites and removes no file; and the lane that drove the next room
/// puts a new stock back in its place. The first room is dispatched the
/// way a person's dispatch goes, through a lane and a landing.
#[test]
fn the_next_room_takes_the_stock_and_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    lay_review_city(dir.path(), &["room1", "room2"]);
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .dispatch_into_lane(
            asked("lab/room1"),
            "work".to_owned(),
            "work".to_owned(),
            Owing::asked(wire::Reply::nowhere()),
        )
        .unwrap();
    worker.land_the_rest().unwrap();

    let flown = fly_here(&mut worker, "lab/room2");
    let Flown::Model { site, driven, .. } = &flown else {
        panic!("a model dispatch flies a model run");
    };
    let placed = site.lease.as_ref().map(storage::WorktreeLease::work);
    assert_eq!(
        (
            driven.is_ok(),
            placed.map(|work| storage::FileWork { walked: 0, ..work }),
            stock_is_ready(dir.path()),
        ),
        (true, Some(storage::FileWork::default()), true),
        "the second room's tree is the stock taken over, and a new stock stands"
    );
}

/// A city with one building under review, `lab`, and the named rooms in
/// it, each holding one file, so the commit the first placement makes
/// carries files a whole checkout has to create.
fn lay_review_city(city_root: &Path, rooms: &[&str]) {
    crate::worker::fixture::init_city(city_root).unwrap();
    for room in rooms {
        let at = city_root.join("lab").join(room);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("notes.md"), format!("# {room}\n")).unwrap();
    }
    lay_rules(city_root, "lab", &ordinary_rules("review = true\n"));
}

/// A dispatch to `room` the person asked for.
fn asked(room: &str) -> Assignment {
    Assignment {
        addr: Address::parse(room).unwrap(),
        session: None,
        effort: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        parent: None,
        succession: None,
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::Person,
        origin: None,
    }
}

/// Stages a dispatch to `room` and flies it on this thread, through the
/// city's own ledger: everything a lane does for the run, and nothing
/// the accounting thread does once it is home.
fn fly_here(worker: &mut RunWorker, room: &str) -> Flown {
    let (staged, _continuation) = worker
        .stage_dispatch(asked(room), "work".to_owned(), "work".to_owned())
        .unwrap();
    let context = worker.drive_context();
    staged.fly(&mut worker.ledger, context)
}

/// Whether the city holds a stock the next placement can take over:
/// registered, its directory in place, and no stocking under way.
fn stock_is_ready(city_root: &Path) -> bool {
    git2::Repository::open(city_root)
        .unwrap()
        .find_worktree("+spare")
        .is_ok_and(|tree| {
            tree.validate().is_ok()
                && matches!(
                    tree.is_locked().unwrap(),
                    git2::WorktreeLockStatus::Unlocked
                )
        })
}
