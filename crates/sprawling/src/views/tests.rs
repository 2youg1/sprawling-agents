// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use crate::assembly::*;
use crate::views::Views;
use kernel::{Address, EventKind, EventRecord, Payload, RunId};

#[test]
fn the_five_views_that_used_to_say_unavailable_answer_from_the_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let run = RunId::from_bytes([7u8; 16]);

    let mut enqueued = serde_json::Map::new();
    enqueued.insert(
        "id".to_owned(),
        serde_json::Value::String("sig-1".to_owned()),
    );
    enqueued.insert(
        "kind".to_owned(),
        serde_json::Value::String("question".to_owned()),
    );
    enqueued.insert(
        "from".to_owned(),
        serde_json::Value::String("lab/room2".to_owned()),
    );
    enqueued.insert(
        "room".to_owned(),
        serde_json::Value::String(room.as_str().to_owned()),
    );
    views
        .apply(&view_record(
            1,
            run,
            EventKind::SignalEnqueued,
            &room,
            enqueued.clone(),
        ))
        .unwrap();

    let channels::Answer::Inbox(inbox) =
        views.answer(&channels::Query::InboxView { addr: room.clone() })
    else {
        panic!("InboxView answers with an inbox");
    };
    assert_eq!(inbox.waiting.len(), 1);
    assert_eq!(inbox.waiting[0].kind, "question");

    // Taking a signal empties the row; the view never took it itself.
    views
        .apply(&view_record(
            2,
            run,
            EventKind::SignalConsumed,
            &room,
            enqueued,
        ))
        .unwrap();
    let channels::Answer::Inbox(inbox) =
        views.answer(&channels::Query::InboxView { addr: room.clone() })
    else {
        panic!("InboxView answers with an inbox");
    };
    assert!(inbox.waiting.is_empty());

    let mut discarded = serde_json::Map::new();
    discarded.insert(
        "paths".to_owned(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "file:lab/room1/notes.md".to_owned(),
        )]),
    );
    let mut way_back = serde_json::Map::new();
    way_back.insert(
        "tracked".to_owned(),
        // A real checkpoint oid, because the plan is parsed rather
        // than copied: `wave_post` writes the commit it fenced, and
        // a locator that does not parse is not a way back.
        serde_json::Value::String(format!("file:lab/room1/notes.md@{}", "ab".repeat(20))),
    );
    discarded.insert(
        "restoration".to_owned(),
        serde_json::Value::Object(way_back),
    );
    views
        .apply(&view_record(
            3,
            run,
            EventKind::FileDiscarded,
            &room,
            discarded.clone(),
        ))
        .unwrap();
    let channels::Answer::Discards(bin) = views.answer(&channels::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert_eq!(bin.rows.len(), 1);
    // The plan travels as a plan: the interface owns the sentence,
    // and a server that composed one too would be the second place
    // that decides what a way back reads like.
    assert!(
        matches!(
            bin.rows[0].restoration,
            Some(channels::Restoration::Tracked(_))
        ),
        "every row states its own way back: {:?}",
        bin.rows[0].restoration
    );
    assert!(!bin.rows[0].restored);

    views
        .apply(&view_record(
            4,
            run,
            EventKind::DiscardRestored,
            &room,
            discarded.clone(),
        ))
        .unwrap();
    let channels::Answer::Discards(bin) = views.answer(&channels::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert!(
        bin.rows[0].restored,
        "a restoration closes the row it opened rather than opening a second one"
    );
    assert_eq!(bin.rows.len(), 1);

    let mut archived = serde_json::Map::new();
    archived.insert(
        "kind".to_owned(),
        serde_json::Value::String("decision".to_owned()),
    );
    archived.insert(
        "subject".to_owned(),
        serde_json::Value::String("chose git over a second index".to_owned()),
    );
    views
        .apply(&view_record(
            5,
            run,
            EventKind::AssetArchived,
            &room,
            archived,
        ))
        .unwrap();
    let channels::Answer::Registry(registry) = views.answer(&channels::Query::RegistryView) else {
        panic!("RegistryView answers with a registry");
    };
    assert_eq!(registry.assets.len(), 1);
    assert_eq!(registry.assets[0].kind, "decision");

    let channels::Answer::Metrics(metrics) = views.answer(&channels::Query::Metrics) else {
        panic!("Metrics answers with the vital signs");
    };
    assert_eq!(metrics.events, 5, "one number no other view can derive");
    assert_eq!(metrics.signals_waiting, 0);
    assert_eq!(
        metrics.discards_outstanding, 0,
        "a restored file is not still missing"
    );

    // The archive is read from the shelves, so a city with no shelf
    // answers an empty search rather than failing to search.
    let channels::Answer::Archive(found) = views.answer(&channels::Query::ArchiveSearch {
        needle: "git".to_owned(),
    }) else {
        panic!("ArchiveSearch answers with hits");
    };
    assert_eq!(found.needle, "git");
    assert!(found.hits.is_empty());

    // A scheme this build cannot read still produces a row. Hiding a
    // discarded thing is worse than admitting the plan is unreadable,
    // and the interface has a sentence for exactly that case.
    let mut unreadable = discarded;
    unreadable.insert(
        "paths".to_owned(),
        serde_json::Value::Array(vec![serde_json::Value::String(
            "file:lab/room1/other.md".to_owned(),
        )]),
    );
    unreadable.insert(
        "restoration".to_owned(),
        serde_json::json!({ "teleported": "somewhere" }),
    );
    views
        .apply(&view_record(
            6,
            run,
            EventKind::FileDiscarded,
            &room,
            unreadable,
        ))
        .unwrap();
    let channels::Answer::Discards(bin) = views.answer(&channels::Query::DiscardView) else {
        panic!("DiscardView answers with the bin");
    };
    assert_eq!(bin.rows.len(), 2, "the unreadable plan still gets a row");
    assert!(
        bin.rows
            .iter()
            .any(|row| row.path.ends_with("other.md") && row.restoration.is_none())
    );
}

/// One record for a view test, carrying a payload and an address.
#[cfg(test)]
pub(super) fn view_record(
    seq: u64,
    run: RunId,
    kind: EventKind,
    addr: &Address,
    data: serde_json::Map<String, serde_json::Value>,
) -> EventRecord {
    EventRecord::from_draft(
        kernel::EventDraft {
            run,
            t: kernel::TimeMs::new(1_000),
            who: "lab/room1".to_owned(),
            addr: Some(addr.clone()),
            kind,
            data: Payload::new(data).unwrap(),
            ig: false,
        },
        kernel::Seq::new(seq),
        kernel::B3Hash::digest(b"prev"),
    )
}

#[test]
fn a_new_building_is_visible_in_the_city_view_with_a_denominator_of_zero() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();

    let mut views = Views::new(dir.path());
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    let lab = city
        .buildings
        .iter()
        .find(|b| b.addr.as_str() == "lab")
        .expect("a building the city made is a building the city can see");
    assert!(lab.problems.is_empty());
    let kernel::Progress::Planned(planned) = lab.progress else {
        panic!("a building with a roadmap has a denominator");
    };
    assert_eq!(
        planned.ratio(),
        (0, 0),
        "a new building owes nothing yet, and owes it out of nothing"
    );
}

#[test]
fn a_roadmap_counts_only_the_rows_that_carry_evidence() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(&building).unwrap();
    let evidence = format!("cas:b3-{}", "ab".repeat(32));
    std::fs::write(
        building.join("Roadmap.md"),
        format!(
            "# Roadmap\n\n| # | Item | Weight | Needs | Status | Evidence |\n\
             |---|---|---|---|---|---|\n\
             | 1 | wired | 1 |  | Done | {evidence} |\n\
             | 2 | claimed | 1 |  | Done |  |\n\
             | 3 | waiting | 1 |  | Awaiting approval |  |\n\
             | 4 | later | 1 |  | not started |  |\n"
        ),
    )
    .unwrap();

    let mut views = Views::new(dir.path());
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    // Two buildings: City Hall, which every city is raised with, and
    // the one this test wrote a plan for.
    let plan = city
        .buildings
        .iter()
        .find(|found| found.addr.as_str() == "lab")
        .expect("the building this test wrote a plan for");
    assert!(plan.problems.is_empty(), "{:?}", plan.problems);
    let kernel::Progress::Planned(planned) = plan.progress else {
        panic!("a building with a roadmap has a denominator");
    };
    // Four rows; one Done with evidence counts; the evidence-free Done
    // stays visible and out of the numerator; awaiting approval reads
    // as blocked; `not started` proves case is not part of the contract.
    assert_eq!(
        (planned.done, planned.blocked, planned.total),
        (1, 1, 4),
        "{planned:?}"
    );
}

#[test]
fn a_roadmap_that_cannot_be_parsed_reports_its_rows_rather_than_a_number() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let building = dir.path().join("lab");
    std::fs::create_dir_all(&building).unwrap();
    std::fs::write(
        building.join("Roadmap.md"),
        "| # | Item | Weight | Needs | Status | Evidence |\n\
         |---|---|---|---|---|---|\n\
         | 1 | x | 1 |  | nearly there |  |\n",
    )
    .unwrap();

    let mut views = Views::new(dir.path());
    let channels::Answer::City(city) = views.answer(&channels::Query::CityView) else {
        panic!("CityView answers with a city");
    };
    let plan = city
        .buildings
        .iter()
        .find(|found| found.addr.as_str() == "lab")
        .expect("the building this test wrote a plan for");
    assert!(plan.problems.iter().any(|p| p.contains("nearly there")));
    assert!(
        matches!(plan.progress, kernel::Progress::Unplanned(_)),
        "an unreadable plan has no denominator, and no percentage"
    );
}

/// The fold takes the same lock a reader does, so a reader that ran
/// `git status` under it held every event behind that walk of the disk.
#[test]
fn a_git_status_reader_does_not_hold_the_views_while_git_reads_the_disk() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let lab = dir.path().join("lab");
    std::fs::create_dir(&lab).unwrap();
    for n in 0..2000 {
        std::fs::write(lab.join(format!("f{n}.txt")), "x").unwrap();
    }
    let views = Arc::new(Mutex::new(Views::new(dir.path())));
    let query = channels::Query::GitStatus {
        building: Address::parse("lab").unwrap(),
    };
    let solo = (0..3)
        .map(|_| {
            let asked = Instant::now();
            let answer = crate::views::answer_outside_the_lock(&views, &query).unwrap();
            assert!(
                matches!(answer, channels::Answer::GitStatus(_)),
                "{answer:?}"
            );
            asked.elapsed()
        })
        .min()
        .unwrap();

    let done = Arc::new(AtomicBool::new(false));
    let reader = {
        let (views, query, done) = (Arc::clone(&views), query.clone(), Arc::clone(&done));
        std::thread::spawn(move || {
            for _ in 0..8 {
                crate::views::answer_outside_the_lock(&views, &query).unwrap();
            }
            done.store(true, Ordering::SeqCst);
        })
    };
    let mut longest = Duration::ZERO;
    while !done.load(Ordering::SeqCst) {
        let asked = Instant::now();
        drop(views.lock().unwrap());
        longest = longest.max(asked.elapsed());
        std::thread::yield_now();
    }
    reader.join().unwrap();
    assert!(
        longest * 4 < solo,
        "the fold waited {longest:?} for the views while one git status takes {solo:?}"
    );
}

/// A page that opens a file or lists a directory reads the tree after
/// the views are released, so it shows the tree as it is at `finish`.
#[test]
fn the_tree_a_page_reads_is_read_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let at = Address::parse("notes").unwrap();
    let document = views.prepare(&channels::Query::Document { at: at.clone() });
    let listing = views.prepare(&channels::Query::Listing { at: None });
    std::fs::write(dir.path().join("notes"), "written after the lock").unwrap();

    assert_eq!(
        (document.finish(), listing.finish()),
        (
            channels::Answer::Document(Box::new(channels::DocumentAnswer {
                at,
                text: "written after the lock".to_owned(),
                bytes: 22,
                truncated: false,
                binary: false,
            })),
            channels::Answer::Listing(channels::ListingAnswer {
                at: None,
                entries: vec![channels::Entry {
                    name: "notes".to_owned(),
                    kind: channels::EntryKind::File { bytes: 22 },
                }],
            }),
        )
    );
}

/// The building page walks the building's directory after the views are
/// released; only its plan, folded and cached, is read under them.
#[test]
fn the_building_page_reads_its_directory_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let query = channels::Query::BuildingView {
        addr: Address::parse("lab").unwrap(),
    };
    let prepared = views.prepare(&query);
    std::fs::create_dir(dir.path().join("lab")).unwrap();

    let answer = prepared.finish();
    assert!(
        matches!(answer, channels::Answer::Building(_)),
        "{answer:?}"
    );
    assert_eq!(answer, views.answer(&query));
}

/// A change list, a patch, a stored object and an archive search read the
/// disk after the views are released: a city that appears between
/// `prepare` and `finish` is the one each of them reports.
#[test]
fn git_store_and_archive_readers_read_after_the_views_are_released() {
    let tmp = tempfile::tempdir().unwrap();
    let (staged, root) = (tmp.path().join("staged"), tmp.path().join("city"));
    let lab = Address::parse("lab").unwrap();
    std::fs::create_dir_all(staged.join("lab")).unwrap();
    let repo = git2::Repository::init(&staged).unwrap();
    let signature = git2::Signature::now("city", "city@localhost").unwrap();
    let commit = |text: &str| {
        std::fs::write(staged.join("lab/lex.rs"), text).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("lab/lex.rs")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parents: Vec<&git2::Commit> = parents.iter().collect();
        let oid = repo
            .commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)
            .unwrap();
        kernel::GitOid::from_bytes(oid.as_bytes().try_into().unwrap())
    };
    let (oid_a, oid_b) = (commit("a\n"), commit("a\nb\n"));
    std::fs::write(staged.join("lab/lex.rs"), "a\nb\nc\n").unwrap();
    let hash = memory::Cas::open(&kernel::layout::CityLayout::new(&staged).cas())
        .unwrap()
        .put(b"held in the store")
        .unwrap();
    let entry = city::archive_entry(
        &staged,
        &lab,
        city::ArchiveKind::parse("decision").unwrap(),
        kernel::TimeMs::new(0),
        "chose git over a second index",
    )
    .unwrap();
    city::file_archive(&entry, "because").unwrap();

    let queries = [
        channels::Query::Changes {
            base: oid_b,
            head: None,
        },
        channels::Query::Hunks {
            oid_a,
            oid_b,
            path: "lab/lex.rs".to_owned(),
        },
        channels::Query::Content {
            locator: kernel::Locator::Cas { hash, range: None },
        },
        channels::Query::ArchiveSearch {
            needle: "git".to_owned(),
        },
    ];
    let mut views = Views::new(&root);
    let prepared: Vec<_> = queries.iter().map(|query| views.prepare(query)).collect();
    // Opening the store makes its directory, so a read done under the
    // lock leaves a directory behind for the city to replace.
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    std::fs::rename(&staged, &root).unwrap();

    let read: Vec<_> = prepared
        .into_iter()
        .map(|prepared| prepared.finish())
        .collect();
    let fresh: Vec<_> = queries.iter().map(|query| views.answer(query)).collect();
    assert_eq!(read, fresh);
    assert!(
        read.iter().all(
            |answer| !matches!(answer, channels::Answer::Unavailable { .. })
                && !matches!(answer, channels::Answer::Archive(found) if found.hits.is_empty())
        ),
        "{read:?}"
    );
}
