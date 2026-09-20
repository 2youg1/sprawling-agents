// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

/// Long enough that one `write_all` reaches the device in several
/// pieces, which is the window a truncating write leaves open.
fn version(mark: char) -> Vec<u8> {
    std::iter::repeat_n(mark, 512 * 1024)
        .collect::<String>()
        .into_bytes()
}

/// The closing condition: a reader that arrives while a document is
/// being replaced holds the old version or the new one, never a
/// prefix of either.
#[test]
fn a_reader_never_meets_half_a_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CONFIG.toml");
    let (old, new) = (version('o'), version('n'));
    replace(&path, &old).unwrap();

    let done = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let writing = scope.spawn(|| {
            for _ in 0..8 {
                replace(&path, &new).unwrap();
                replace(&path, &old).unwrap();
            }
            done.store(true, Ordering::Release);
        });
        while !done.load(Ordering::Acquire) {
            let seen = std::fs::read(&path).unwrap();
            assert!(
                seen == old || seen == new,
                "a reader saw {} bytes, which is neither version",
                seen.len()
            );
        }
        writing.join().unwrap();
    });
}

/// A writer killed between the flush and the rename leaves a staging
/// file behind. It is not the document, and the next write reuses it
/// rather than refusing or piling up a second one.
#[test]
fn a_staging_file_left_by_a_killed_writer_is_not_the_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("BUILDING.md");
    replace(&path, b"# the rules as they stand\n").unwrap();
    let staged = staging_path(&path).unwrap();
    std::fs::write(&staged, b"# half a rul").unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# the rules as they stand\n"
    );
    replace(&path, b"# the rules a person just saved\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# the rules a person just saved\n"
    );
    assert!(
        !staged.exists(),
        "the staging file survived the write that consumed it"
    );
}

/// Two writers of one document take turns: each reads what the other
/// wrote, so neither change is lost. Without the lock both read the
/// same original and the second rename erases the first's work.
#[test]
fn two_writers_of_one_document_do_not_overwrite_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CONFIG.toml");
    replace(&path, b"0").unwrap();

    let raise = || {
        for _ in 0..200 {
            edit(&path, |held| {
                let held_count: u32 = std::fs::read_to_string(&path).unwrap().parse().unwrap();
                held.replace(format!("{}", held_count + 1).as_bytes())
            })
            .unwrap();
        }
    };
    std::thread::scope(|scope| {
        scope.spawn(raise);
        scope.spawn(raise);
    });

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "400",
        "a change was read, decided on, and then written over"
    );
}

/// The directories above a document are made by the write, because
/// every caller in the city was making them itself beforehand.
#[test]
fn a_document_brings_its_directories_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lab").join("room1").join("JOB.md");
    replace(&path, b"# the task\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# the task\n");
}
