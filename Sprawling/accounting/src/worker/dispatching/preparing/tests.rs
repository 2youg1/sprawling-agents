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

#[test]
fn a_lane_puts_the_stock_back_after_its_run_comes_home() {
    let dir = tempfile::tempdir().unwrap();
    lay_review_city(dir.path(), &["room1"]);
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();

    let (flown, stocked_at_home) = fly_here(&mut worker, "lab/room1");
    let Flown::Model { site, driven, .. } = &flown else {
        panic!("a model dispatch flies a model run");
    };
    assert_eq!(
        (
            driven.is_ok(),
            site.lease.is_some(),
            stocked_at_home,
            stock_is_ready(dir.path())
        ),
        (true, true, false, true),
        "the run drove in a tree it borrowed, came home, and then its lane put the stock back"
    );
}

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

    let stocked = (0..2_000).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(5));
        stock_is_ready(dir.path())
    });
    assert!(stocked, "the first room's lane put the stock back");

    let (flown, _) = fly_here(&mut worker, "lab/room2");
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

        std::mem::forget(first);
        false
    };
    assert_eq!(
        (answered, again),
        (Ok((false, true)), true),
        "the second lane skips without taking the first lane's turn away"
    );
}

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
        let mut homed = None;
        staged.fly(&mut timed, context, |flown| {
            homed = Some((Instant::now(), flown));
        });
        let ended = Instant::now();
        let (home, flown) = homed.unwrap();
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
            "instrument_production_placement room={room} created={created} placement_ms={} landing_wait_ms={} restock_ms={} fly_ms={} machine={}-{}, {cores} core(s)",
            ms(began, timed.opened.unwrap()),
            ms(timed.last.unwrap(), home),
            ms(home, ended),
            ms(began, ended),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
    }
}

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

fn lay_review_city(city_root: &Path, rooms: &[&str]) {
    crate::worker::fixture::init_city(city_root).unwrap();
    for room in rooms {
        let at = city_root.join("lab").join(room);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("notes.md"), format!("# {room}\n")).unwrap();
    }
    lay_rules(city_root, "lab", &ordinary_rules("review = true\n"));
}

fn asked(room: &str) -> Assignment {
    Assignment {
        addr: Address::parse(room).unwrap(),
        session: None,
        effort: None,
        difficulty: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        parent: None,
        succession: None,
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::Person,
        origin: None,
    }
}

fn fly_here(worker: &mut RunWorker, room: &str) -> (Flown, bool) {
    let (staged, _continuation) = worker
        .stage_dispatch(asked(room), "work".to_owned(), "work".to_owned())
        .unwrap();
    let context = worker.drive_context();
    let city_root = worker.city_root.clone();
    let mut homed = None;
    staged.fly(&mut worker.ledger, context, |flown| {
        homed = Some((flown, stock_is_ready(&city_root)));
    });
    homed.unwrap()
}

fn stock_is_ready(city_root: &Path) -> bool {
    let Ok(repo) = git2::Repository::open(city_root) else {
        return false;
    };
    let Ok(tree) = repo.find_worktree("+spare") else {
        return false;
    };
    tree.validate().is_ok() && matches!(tree.is_locked(), Ok(git2::WorktreeLockStatus::Unlocked))
}

#[test]
fn tp4_a_lane_keeps_the_transcript_before_its_run_comes_home() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab").join("room1")).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let (staged, _continuation) = worker
        .stage_dispatch(asked("lab/room1"), "work".to_owned(), "work".to_owned())
        .unwrap();
    let transcript = dir
        .path()
        .join("lab")
        .join("room1")
        .join(format!("{}.jsonl", staged.run_id()));
    let context = worker.drive_context();
    let mut kept_at_home = None;
    staged.fly(&mut worker.ledger, context, |_flown| {
        kept_at_home = Some(transcript.is_file());
    });
    assert_eq!(
        kept_at_home,
        Some(true),
        "the transcript is on disk when the run comes home"
    );
}
