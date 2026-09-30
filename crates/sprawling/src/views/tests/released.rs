// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::views::Views;
use kernel::Address;

/// The fold takes the same lock a reader does, so a reader that ran
/// `git status` under it held every event behind that walk of the disk:
/// a file written between `prepare` and `finish` is one the answer shows.
#[test]
fn a_git_status_reader_does_not_hold_the_views_while_git_reads_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let lab = dir.path().join("lab");
    std::fs::create_dir(&lab).unwrap();
    std::fs::write(lab.join("before.txt"), "x").unwrap();
    let mut views = Views::new(dir.path());
    let query = wire::Query::GitStatus {
        building: Address::parse("lab").unwrap(),
    };
    let prepared = views.prepare(&query);
    std::fs::write(lab.join("after.txt"), "y").unwrap();

    let read = prepared.finish();
    assert!(matches!(read, wire::Answer::GitStatus(_)), "{read:?}");
    assert_eq!(read, views.answer(&query));
}

/// The configuration ladder is read after the views are released, and
/// the release page is not asked for under them: `prepare` leaves both
/// reads to `finish`.
#[test]
fn the_config_ladder_and_the_release_page_are_read_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let room = Address::parse("lab/room1").unwrap();
    let query = wire::Query::Config { addr: room.clone() };
    let prepared = views.prepare(&query);
    let city_layer = city::config_path(dir.path(), &room, city::Layer::City).unwrap();
    std::fs::create_dir_all(city_layer.parent().unwrap()).unwrap();
    std::fs::write(
        &city_layer,
        "[model]
effort = \"low\"
",
    )
    .unwrap();

    let read = prepared.finish();
    assert_eq!(read, views.answer(&query));
    let wire::Answer::Config(config) = read else {
        panic!("Config answers with a ladder");
    };
    assert!(config.effort.is_some(), "{config:?}");
    assert!(matches!(
        views.prepare(&wire::Query::NewestRelease),
        crate::views::prepared::Prepared::Release(_)
    ));
}

/// A page that opens a file or lists a directory reads the tree after
/// the views are released, so it shows the tree as it is at `finish`.
#[test]
fn the_tree_a_page_reads_is_read_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let views = Views::new(dir.path());
    let at = Address::parse("notes").unwrap();
    let document = views.prepare(&wire::Query::Document { at: at.clone() });
    let listing = views.prepare(&wire::Query::Listing { at: None });
    std::fs::write(dir.path().join("notes"), "written after the lock").unwrap();

    assert_eq!(
        (document.finish(), listing.finish()),
        (
            wire::Answer::Document(Box::new(wire::DocumentAnswer {
                at,
                text: "written after the lock".to_owned(),
                bytes: 22,
                truncated: false,
                binary: false,
            })),
            wire::Answer::Listing(wire::ListingAnswer {
                at: None,
                entries: vec![wire::Entry {
                    name: "notes".to_owned(),
                    kind: wire::EntryKind::File { bytes: 22 },
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
    let query = wire::Query::BuildingView {
        addr: Address::parse("lab").unwrap(),
    };
    let prepared = views.prepare(&query);
    std::fs::create_dir(dir.path().join("lab")).unwrap();

    let answer = prepared.finish();
    assert!(matches!(answer, wire::Answer::Building(_)), "{answer:?}");
    assert_eq!(answer, views.answer(&query));
}

/// The city page lists the buildings and reads their plans after the
/// views are released: a building raised between `prepare` and `finish`
/// is one the answer shows, and its plan is the one on disk then.
#[test]
fn the_city_page_lists_its_buildings_and_reads_their_plans_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let views = Views::new(dir.path());
    let prepared = views.prepare(&wire::Query::CityView);
    std::fs::create_dir(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab").join("Roadmap.md"),
        "| # | Item | Weight | Needs | Status | Evidence |\n\
         |---|------|--------|-------|--------|----------|\n\
         | 1 | groundwork | 1 |  | Not started |  |\n",
    )
    .unwrap();

    let read = prepared.finish();
    let wire::Answer::City(city) = &read else {
        panic!("CityView answers with a city: {read:?}");
    };
    assert_eq!(
        city.buildings
            .iter()
            .map(|line| (line.addr.as_str().to_owned(), line.ready))
            .collect::<Vec<_>>(),
        vec![("lab".to_owned(), 1)]
    );
    assert_eq!(read, views.prepare(&wire::Query::CityView).finish());
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
    let hash = storage::Cas::open(&kernel::layout::CityLayout::new(&staged).cas())
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
        wire::Query::Changes {
            base: oid_b,
            head: None,
        },
        wire::Query::Hunks {
            oid_a,
            oid_b,
            path: "lab/lex.rs".to_owned(),
        },
        wire::Query::Content {
            locator: kernel::Locator::Cas { hash, range: None },
        },
        wire::Query::ArchiveSearch {
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
        read.iter()
            .all(|answer| !matches!(answer, wire::Answer::Unavailable { .. })
                && !matches!(answer, wire::Answer::Archive(found) if found.hits.is_empty())),
        "{read:?}"
    );
}

/// The skills page scans the shelves, and the vital signs count the
/// buildings, after the views are released: a building raised between
/// `prepare` and `finish` is one each of them sees.
#[test]
fn the_shelves_and_the_building_count_are_read_after_the_views_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let queries = [
        wire::Query::Skills {
            building: Address::parse("lab").unwrap(),
        },
        wire::Query::Metrics,
    ];
    let prepared: Vec<_> = queries.iter().map(|query| views.prepare(query)).collect();
    let shelf = kernel::layout::CityLayout::new(dir.path())
        .building_skills(&Address::parse("lab").unwrap())
        .join("utilities");
    std::fs::create_dir_all(&shelf).unwrap();
    std::fs::write(
        shelf.join("diffing.md"),
        "This lab's own rule
",
    )
    .unwrap();

    let read: Vec<_> = prepared
        .into_iter()
        .map(|prepared| prepared.finish())
        .collect();
    let fresh: Vec<_> = queries.iter().map(|query| views.answer(query)).collect();
    assert_eq!(read, fresh);
}
