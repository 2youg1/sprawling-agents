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
use std::time::Instant;

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

/// sprawling-SPEC.md 8-161: a run lands while its lane is still putting
/// the stock back. The test holds the table of cities being stocked, so
/// the lane's restock waits on it; the run still comes home through a
/// real lane and lands, with no stock in the city yet. Once the table is
/// let go, closing the worker waits for that lane, and the stock stands.
/// The bound on serving only keeps a lane that never comes home from
/// hanging the test.
#[test]
fn a_run_lands_while_its_lane_still_puts_the_stock_back() {
    let dir = tempfile::tempdir().unwrap();
    lay_review_city(dir.path(), &["room1"]);
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let held = STOCKING.lock().unwrap_or_else(PoisonError::into_inner);
    worker
        .dispatch_into_lane(
            asked("lab/room1"),
            "work".to_owned(),
            "work".to_owned(),
            Owing::asked(wire::Reply::nowhere()),
        )
        .unwrap();
    let landed = (0..2_000).any(|_| {
        worker
            .serve_flight(relay::Patience::For(std::time::Duration::from_millis(5)))
            .unwrap();
        !worker.driving()
    });
    let stocked_at_landing = stock_is_ready(dir.path());
    drop(held);
    drop(worker);
    assert_eq!(
        (landed, stocked_at_landing, stock_is_ready(dir.path())),
        (true, false, true),
        "the run landed before its lane put the stock back, and the stock stands once the worker closed"
    );
}

/// sprawling-SPEC.md 8-155: a lane that finds the city's turn taken
/// skips, and the turn it found stays with the lane that holds it; once
/// that lane gives it back, the city's turn can be taken again. Two
/// lanes of one city stocking at once is what the acceptance catalogue's
/// builder run does, and the second take used to build a turn and drop
/// it under the table's own lock, which hung both lanes.
#[test]
fn a_lane_that_finds_the_turn_taken_skips_and_leaves_it_standing() {
    let city = tempfile::tempdir().unwrap();
    let root = city.path().to_path_buf();
    let first = StockingTurn::take(&root).expect("the first lane takes the turn");
    let (told, heard) = std::sync::mpsc::channel();
    let probe = root.clone();
    std::thread::spawn(move || {
        let second = StockingTurn::take(&probe).is_some();
        let standing = STOCKING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&probe);
        told.send((second, standing)).unwrap();
    });
    let answered = heard.recv_timeout(std::time::Duration::from_secs(10));
    let again = if answered.is_ok() {
        drop(first);
        StockingTurn::take(&root).is_some()
    } else {
        // The second lane hangs holding the table's lock, so giving the
        // first turn back would hang this thread as well.
        std::mem::forget(first);
        false
    };
    assert_eq!(
        (answered, again),
        (Ok((false, true)), true),
        "the second lane skips without taking the first lane's turn away"
    );
}

/// Reads production placement and the stock put back after it, in
/// milliseconds, over a building whose trunk carries 512 files of 16 KB
/// (sprawling-SPEC.md 8-155). The first room's placement makes the
/// city's first commit and checks the trunk out whole; the second room's
/// takes the stock over. Each lane then puts a stock back, and that is
/// what lies between the run's last line and `fly` returning.
#[test]
#[ignore = "a wall-clock instrument; prints one reading line per room"]
#[allow(
    clippy::disallowed_methods,
    reason = "an instrument reads the wall clock"
)]
fn instrument_production_placement() {
    let dir = tempfile::tempdir().unwrap();
    lay_review_city(dir.path(), &["room1", "room2"]);
    let bulk = dir.path().join("lab").join("room1").join("bulk");
    std::fs::create_dir_all(&bulk).unwrap();
    for file in 0u64..512 {
        let mut bytes = vec![b'.'; 16 * 1024];
        bytes[..8].copy_from_slice(&file.to_le_bytes());
        std::fs::write(bulk.join(format!("file-{file:04}.txt")), bytes).unwrap();
    }
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("first", None), completion("second", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    for room in ["lab/room1", "lab/room2"] {
        let (staged, _continuation) = worker
            .stage_dispatch(asked(room), "work".to_owned(), "work".to_owned())
            .unwrap();
        let context = worker.drive_context();
        let mut timed = Timed {
            ledger: &mut worker.ledger,
            opened: None,
            last: None,
        };
        let began = Instant::now();
        let flown = staged.fly(&mut timed, context);
        let ended = Instant::now();
        let Flown::Model { site, driven, .. } = &flown else {
            panic!("a model dispatch flies a model run");
        };
        assert!(
            driven.is_ok(),
            "the run in {room} drove: {:?}",
            driven.as_ref().err()
        );
        let created = site.lease.as_ref().unwrap().work().created;
        let ms = |from: Instant, to: Instant| to.saturating_duration_since(from).as_millis();
        println!(
            "instrument_production_placement room={room} created={created} placement_ms={} restock_ms={} fly_ms={} machine={}-{}, {cores} core(s)",
            ms(began, timed.opened.unwrap()),
            ms(timed.last.unwrap(), ended),
            ms(began, ended),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
    }
}

/// A ledger that keeps the moment its `worktree_opened` line and its
/// last line were written, over the one it writes to.
struct Timed<'a, L> {
    ledger: &'a mut L,
    opened: Option<Instant>,
    last: Option<Instant>,
}

impl<L: kernel::Ledger> kernel::Ledger for Timed<'_, L> {
    #[allow(
        clippy::disallowed_methods,
        reason = "an instrument reads the wall clock"
    )]
    fn append(&mut self, draft: kernel::EventDraft) -> Result<kernel::EventRef, AxError> {
        let kind = draft.kind;
        let appended = self.ledger.append(draft);
        let at = Instant::now();
        if kind == EventKind::WorktreeOpened {
            self.opened = Some(at);
        }
        self.last = Some(at);
        appended
    }
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
