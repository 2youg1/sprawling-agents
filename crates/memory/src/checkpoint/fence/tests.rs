// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::provenance::{ModelChoice, Provenance};
use super::*;
use kernel::{Address, B3Hash, Effort, RunId};

pub(crate) fn resident() -> Provenance {
    Provenance::new(
        RunId::parse("018f5b2a-0000-7000-8000-000000000001").unwrap(),
        Address::parse("work/resident").unwrap(),
        B3Hash::digest(b"a city"),
        ModelChoice {
            id: "test-model".to_owned(),
            effort: Some(Effort::Medium),
        },
    )
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}
fn oid_of(payload: &Payload) -> String {
    serde_json::to_value(payload).unwrap()["oid"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn files_of(payload: &Payload) -> Vec<String> {
    serde_json::to_value(payload).unwrap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_fence_covers_every_prefix_it_is_given_and_nothing_else() {
    // The defect this pins: a run's fence used to be its room, while the
    // write domain it is judged against is the building and whatever
    // else that building declares. Anything the run was allowed to write
    // and the fence did not stage was invisible to `changes`, and
    // `wave_post` could not restore it either.
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/resident/in-room.txt", "a");
    write(
        tmp.path(),
        "work/note.md",
        "outside the room, inside the domain",
    );
    write(tmp.path(), "shared/also-declared.txt", "a second prefix");
    write(tmp.path(), "other/elsewhere.txt", "outside the domain");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();

    let pre = checkpoint
        .wave_pre(
            &["work".to_owned(), "shared".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();

    let files = files_of(&pre);
    for expected in [
        "work/resident/in-room.txt",
        "work/note.md",
        "shared/also-declared.txt",
    ] {
        assert!(files.iter().any(|f| f == expected), "{expected}: {files:?}");
    }
    assert!(
        !files.iter().any(|f| f == "other/elsewhere.txt"),
        "a prefix nobody declared stays out: {files:?}"
    );
}

#[test]
fn a14_the_fence_precedes_the_deletion_it_restores() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/keep.txt", "kept");
    write(tmp.path(), "work/doomed.txt", "about to go");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();

    let pre = checkpoint
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();
    let pre_oid = oid_of(&pre);
    let files = serde_json::to_value(&pre).unwrap();
    assert_eq!(files["files"].as_array().unwrap().len(), 2);

    // The wave deletes a file.
    std::fs::remove_file(tmp.path().join("work/doomed.txt")).unwrap();

    let discards = checkpoint.wave_post(&pre_oid).unwrap();
    assert_eq!(discards.len(), 1);
    let value = serde_json::to_value(&discards[0]).unwrap();
    assert_eq!(value["paths"][0], "file:work/doomed.txt");
    assert_eq!(
        value["restoration"]["tracked"],
        format!("file:work/doomed.txt@{pre_oid}"),
        "the restoration names a commit that already exists"
    );
}

#[test]
fn the_scope_is_the_boundary_and_outside_it_nothing_is_staged() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/mine.txt", "in domain");
    write(tmp.path(), "elsewhere/theirs.txt", "not mine");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let pre = checkpoint
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();
    let files = serde_json::to_value(&pre).unwrap();
    let staged: Vec<String> = files["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(staged, vec!["work/mine.txt".to_owned()]);
}

#[test]
fn a_wave_pays_for_what_it_changed_rather_than_for_the_whole_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
    write(tmp.path(), "work/smuggled.env", &format!("KEY={token}"));
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    checkpoint.stage_scopes(&["work".to_owned()]).unwrap();
    checkpoint
        .commit(&crate::checkpoint::scan::CommitPlan {
            t: TimeMs::new(1_000),
            of: &resident(),
            subject: "past the fence",
            onto_head: true,
        })
        .unwrap();

    // An ordinary wave that touches a different file. What it pays
    // for is its own change; the blob it did not touch is not read.
    write(tmp.path(), "work/ordinary.txt", "nothing to see");
    checkpoint
        .wave_pre(&["work".to_owned()], TimeMs::new(2_000), &resident())
        .expect("an unchanged blob is not re-examined");

    // And the guard still bites on this wave's own writing, which is
    // the half of the property the narrowing must not cost.
    write(tmp.path(), "work/fresh.env", &format!("KEY={token}"));
    let err = match checkpoint.wave_pre(&["work".to_owned()], TimeMs::new(3_000), &resident()) {
        Err(err) => err,
        Ok(_) => panic!("a secret arriving with this wave must refuse the commit"),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("work/fresh.env"), "{rendered}");
    assert!(
        !rendered.contains("work/smuggled.env"),
        "only what this wave staged is reported: {rendered}"
    );
}

#[test]
fn an_unchanged_wave_still_commits_so_the_chain_rebuilds() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/steady.txt", "unchanged");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let first = oid_of(
        &checkpoint
            .wave_pre(&["work".to_owned()], TimeMs::new(1_000), &resident())
            .unwrap(),
    );
    let second = oid_of(
        &checkpoint
            .wave_pre(&["work".to_owned()], TimeMs::new(2_000), &resident())
            .unwrap(),
    );
    assert_ne!(first, second, "each fence is its own commit");
    assert!(checkpoint.wave_post(&second).unwrap().is_empty());
}

#[test]
fn the_same_script_at_the_same_time_produces_the_same_commit() {
    let build = |dir: &Path| -> String {
        write(dir, "work/a.txt", "alpha");
        let mut checkpoint = Checkpoint::open(dir).unwrap();
        oid_of(
            &checkpoint
                .wave_pre(
                    &["work".to_owned()],
                    TimeMs::new(1_700_000_000_000),
                    &resident(),
                )
                .unwrap(),
        )
    };
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    assert_eq!(
        build(one.path()),
        build(two.path()),
        "time is a parameter, so the oid is reproducible"
    );
}

/// Three tool waves leave three fences a run can restore from, and
/// leave `git log HEAD` exactly as long as it was.
#[test]
fn three_waves_leave_three_fences_and_a_history_that_did_not_grow() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/steady.txt", "unchanged");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let of = resident();
    checkpoint
        .ensure_base(&["work".to_owned()], TimeMs::new(1_000), &of)
        .unwrap();
    let before = head_len(tmp.path());

    let mut fences = Vec::new();
    for step in 0..3u64 {
        write(tmp.path(), "work/steady.txt", &format!("wave {step}"));
        let payload = checkpoint
            .wave_pre(&["work".to_owned()], TimeMs::new(2_000 + step), &of)
            .unwrap();
        fences.push(oid_of(&payload));
    }

    assert_eq!(
        head_len(tmp.path()),
        before,
        "a wave fence does not land on the branch"
    );

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let prefix = format!("refs/sprawling/runs/{}/", of.run());
    let mut filed: Vec<String> = repo
        .references()
        .unwrap()
        .names()
        .flatten()
        .filter(|name| name.starts_with(&prefix))
        .map(str::to_owned)
        .collect();
    filed.sort();
    assert_eq!(
        filed,
        vec![
            format!("{prefix}0"),
            format!("{prefix}1"),
            format!("{prefix}2"),
        ]
    );

    // Each fence still checks out: a dangling commit reachable through
    // its reference is not collected.
    for (step, oid) in fences.iter().enumerate() {
        let tree = repo
            .find_commit(git2::Oid::from_str(oid).unwrap())
            .unwrap()
            .tree()
            .unwrap();
        let entry = tree.get_path(Path::new("work/steady.txt")).unwrap();
        let blob = repo.find_blob(entry.id()).unwrap();
        assert_eq!(blob.content(), format!("wave {step}").as_bytes());
    }
}

/// How many commits `git log HEAD` would print.
fn head_len(root: &Path) -> usize {
    let repo = git2::Repository::open(root).unwrap();
    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.count()
}
