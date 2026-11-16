// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `restore` refuses to overwrite and where it refuses to write:
//! a file the person made after the discard, a link on the path, and an
//! address the city reserves.

use super::tests::{oid_of, resident, write};
use super::*;
use kernel::Address;

#[test]
fn restore_leaves_a_file_the_person_made_after_the_discard() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/doomed.txt", "about to go");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let pre = checkpoint
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();
    let pre_oid = GitOid::parse(&oid_of(&pre)).unwrap();
    std::fs::remove_file(tmp.path().join("work/doomed.txt")).unwrap();
    write(tmp.path(), "work/doomed.txt", "the person's newer file");

    let restored = checkpoint.restore(&Address::parse("work/doomed.txt").unwrap(), &pre_oid);

    assert_eq!(
        (
            restored.is_err(),
            std::fs::read_to_string(tmp.path().join("work/doomed.txt")).unwrap()
        ),
        (true, "the person's newer file".to_owned())
    );
}

#[test]
fn restore_refuses_a_link_on_the_path_and_writes_nothing_outside_the_city() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/doomed.txt", "about to go");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let pre = checkpoint
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();
    let pre_oid = GitOid::parse(&oid_of(&pre)).unwrap();
    std::fs::remove_dir_all(tmp.path().join("work")).unwrap();
    assert!(
        crate::alias::tests::place_link(false, outside.path(), &tmp.path().join("work")),
        "the fixture could not make a link"
    );

    let restored = checkpoint.restore(&Address::parse("work/doomed.txt").unwrap(), &pre_oid);

    assert_eq!(
        (
            restored.is_err(),
            outside.path().join("doomed.txt").exists()
        ),
        (true, false)
    );
}

#[test]
fn restore_refuses_a_reserved_address_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), ".sprawling/doc.txt", "from history");
    let repo = git2::Repository::init(tmp.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".sprawling/doc.txt")).unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("t", "t@t").unwrap();
    let held = repo
        .commit(None, &sig, &sig, "holds a reserved file", &tree, &[])
        .unwrap();
    let checkpoint = Checkpoint::open(tmp.path()).unwrap();
    std::fs::remove_file(tmp.path().join(".sprawling/doc.txt")).unwrap();

    let restored = checkpoint.restore(
        &Address::parse(".sprawling/doc.txt").unwrap(),
        &GitOid::parse(&held.to_string()).unwrap(),
    );

    assert_eq!(
        (
            restored.is_err(),
            tmp.path().join(".sprawling/doc.txt").exists()
        ),
        (true, false)
    );
}

/// Taking a file back replaces the bytes the tree holds with the ones the
/// checkpoint holds, and a file the checkpoint never held is taken away.
#[test]
fn taking_a_file_back_replaces_what_the_tree_holds_and_removes_what_the_point_did_not_hold() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/kept.txt", "as it was");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let pre = checkpoint
        .wave_pre(
            &["work".to_owned()],
            TimeMs::new(1_700_000_000_000),
            &resident(),
        )
        .unwrap();
    let point = GitOid::parse(&oid_of(&pre)).unwrap();
    write(tmp.path(), "work/kept.txt", "changed since");
    write(tmp.path(), "work/new.txt", "made since");

    let kept = checkpoint.take_back(&Address::parse("work/kept.txt").unwrap(), &point);
    let new = checkpoint.take_back(&Address::parse("work/new.txt").unwrap(), &point);

    assert_eq!(
        (
            kept.map(|restored| restored.path).ok(),
            new.is_ok(),
            std::fs::read_to_string(tmp.path().join("work/kept.txt")).unwrap(),
            tmp.path().join("work/new.txt").exists(),
        ),
        (
            Some("work/kept.txt".to_owned()),
            true,
            "as it was".to_owned(),
            false
        )
    );
}
