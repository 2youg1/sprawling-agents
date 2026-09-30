// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::assembly::*;

/// Adopting a folder fences it at once, as one pack: the first dispatch
/// into a building of thousands of files would otherwise hash every one
/// of them and write each as a loose object before its first tool call
/// (storage-SPEC 8-8, the base fence).
#[test]
fn adopting_a_folder_fences_it_as_one_pack_before_any_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.md", "b.md", "c.md"] {
        std::fs::create_dir_all(dir.path().join("shop")).unwrap();
        std::fs::write(dir.path().join("shop").join(name), name).unwrap();
    }
    form_city(dir.path(), Adopt::EveryFolder).unwrap();

    let carried = git2::Repository::open(dir.path()).ok().is_some_and(|repo| {
        repo.head()
            .and_then(|head| head.peel_to_tree())
            .is_ok_and(|tree| tree.get_path(std::path::Path::new("shop/b.md")).is_ok())
    });
    let objects = dir.path().join(".git").join("objects");
    let loose = std::fs::read_dir(&objects)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().len() == 2)
                .count()
        })
        .unwrap_or(0);
    let packs = std::fs::read_dir(objects.join("pack"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pack"))
                .count()
        })
        .unwrap_or(0);
    assert_eq!((carried, loose, packs), (true, 0, 1));
}

/// A city formed around a project's own folder keeps its records beside
/// the project's files, and the project's git would list the ledger and
/// the object store as untracked work. Genesis adds one anchored line
/// for the city's reserved subtree to the root's `.gitignore`, keeping
/// every line the project already had (city-SPEC.md 8-21).
#[test]
fn forming_a_city_keeps_its_own_subtree_out_of_the_workspaces_git() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    form_city(dir.path(), Adopt::EveryFolder).unwrap();

    let ignored = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap_or_default();
    assert!(
        ignored.starts_with("target/\n"),
        "the project's own lines moved: {ignored}"
    );
    assert!(
        ignored.lines().any(|line| line.trim() == "/.sprawling/"),
        "the city's own subtree is not ignored at the root: {ignored}"
    );
    // Once, however often a city is formed or opened here.
    assert_eq!(ignored.matches("/.sprawling/\n").count(), 1);
}

/// No working document and no conversation record the city writes in a
/// project folder is visible to that project's git (city-SPEC.md 12.5).
/// A dispatch sent straight to a room address nobody opened writes its
/// task and its transcript in a room the city made; one sent to a
/// directory the project already had writes them among the project's
/// files, which the city must not seal, so a new source file there is
/// still work git sees.
#[test]
fn a_dispatch_leaves_nothing_the_projects_git_would_pick_up_as_work() {
    use crate::assembly::fixture::{completion, fake_openai, worker_with_provider};
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("proj").join("src")).unwrap();
    std::fs::write(dir.path().join("proj").join("src").join("lib.rs"), "\n").unwrap();
    form_city(dir.path(), Adopt::EveryFolder).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    for (room, idem) in [("proj/room1", b"first"), ("proj/src", b"other")] {
        worker
            .handle(wire::Command::Dispatch {
                addr: kernel::Address::parse(room).unwrap(),
                task: "measure the thing".to_owned(),
                goal: "a number, then stop".to_owned(),
                mode: kernel::Mode::PlanGoal,
                idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, idem),
                session: None,
                effort: None,
                model: None,
            })
            .unwrap();
    }
    assert!(dir.path().join("proj").join("src").join("JOB.md").is_file());
    std::fs::write(dir.path().join("proj").join("src").join("new.rs"), "\n").unwrap();

    let repo = git2::Repository::open(dir.path()).unwrap();
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let visible: Vec<String> = repo
        .statuses(Some(&mut options))
        .unwrap()
        .iter()
        .map(|entry| entry.path().unwrap().to_owned())
        .collect();
    let records: Vec<&String> = visible
        .iter()
        .filter(|path| {
            path.starts_with(".sprawling/")
                || path.starts_with("proj/room1/")
                || path.ends_with(".jsonl")
                || [
                    "JOB.md",
                    "URBANITE.md",
                    "Handoff.md",
                    "Roadmap.md",
                    "Memo.md",
                ]
                .iter()
                .any(|name| path.ends_with(name))
                || path.contains("/Archive/")
        })
        .collect();
    assert!(
        records.is_empty(),
        "git sees the city's working records: {records:?}"
    );
    assert!(
        visible.iter().any(|path| path == "proj/src/new.rs"),
        "the project's own new file is hidden from git: {visible:?}"
    );
}

/// A building raised before a rule existed gets that rule the next time
/// the city opens, so the ruling holds for a city already running.
#[test]
fn opening_a_city_gives_an_older_building_the_rules_it_lacks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("proj")).unwrap();
    form_city(dir.path(), Adopt::EveryFolder).unwrap();
    let ignore = dir.path().join("proj").join(".gitignore");
    std::fs::write(&ignore, "Roadmap.md\nMemo.md\n").unwrap();

    RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    let rules = std::fs::read_to_string(&ignore).unwrap();
    assert!(
        rules.starts_with("Roadmap.md\nMemo.md\n") && rules.lines().any(|line| line == "JOB.md"),
        "{rules}"
    );
}
