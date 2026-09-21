// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a fence stages, scans and commits: the scope boundary, the
//! staged-secret refusal, and the boundary an empty scope means.

use super::super::provenance::{ModelChoice, Provenance};
use super::*;
use kernel::{Address, B3Hash, Effort, RunId};
use std::path::Path;

fn resident() -> Provenance {
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

#[test]
fn a_staged_secret_refuses_the_commit_and_never_echoes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
    write(tmp.path(), "work/leak.env", &format!("KEY={token}"));
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let err = match checkpoint.wave_pre(&["work".to_owned()], TimeMs::new(0), &resident()) {
        Err(err) => err,
        Ok(_) => panic!("a staged secret must refuse the commit"),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("work/leak.env"), "{rendered}");
    assert!(
        !rendered.contains(&token) && !rendered.contains("Zx9yQ2mK4pL7"),
        "positions only, never the bytes: {rendered}"
    );
    let ax = err.into_ax();
    assert_eq!(*ax.code(), kernel::AxCode::SecretEgress);
}

/// Five trailers, in the order and the spelling a reader outside the
/// city was promised. Read as git reads them: the message is parsed
/// line by line off the commit object, which is what
/// `git interpret-trailers --parse` walks.
#[test]
fn every_commit_the_city_makes_names_the_session_that_made_it() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/note.md", "a line");
    let mut checkpoint = Checkpoint::open(tmp.path()).unwrap();
    let of = resident();
    checkpoint
        .ensure_base(&["work".to_owned()], TimeMs::new(1_000), &of)
        .unwrap();

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    let message = commit.message().unwrap();
    let trailers: Vec<String> = message
        .lines()
        .filter(|line| line.starts_with("Sprawling-"))
        .map(str::to_owned)
        .collect();
    let city = kernel::B3Hash::digest(b"a city").to_string();
    assert_eq!(
        trailers,
        vec![
            format!("Sprawling-Run: {}", of.run()),
            "Sprawling-Actor: work/resident".to_owned(),
            "Sprawling-Model: test-model".to_owned(),
            "Sprawling-Effort: medium".to_owned(),
            format!("Sprawling-City: {city}"),
        ]
    );
    assert!(message.starts_with("checkpoint: work\n\n"), "{message}");

    let author = commit.author();
    assert_eq!(author.name().unwrap(), "work/resident");
    assert_eq!(
        author.email().unwrap(),
        format!("work/resident@{}.sprawling", city.get(..12).unwrap())
    );
}

/// A restart must not open a window a credential can walk through.
/// A new process remembers no fence, so the scan compares against
/// HEAD - which a wave fence deliberately does not move, and which is
/// therefore an ancestor of the last fence. Comparing against an
/// ancestor reads more, never less, so what changed before the
/// restart is still read.
#[test]
fn a_change_made_before_a_restart_is_still_read_after_it() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "work/clean.md", "nothing here");
    let mut first = Checkpoint::open(tmp.path()).unwrap();
    first
        .ensure_base(&["work".to_owned()], TimeMs::new(0), &resident())
        .unwrap();
    first
        .wave_pre(&["work".to_owned()], TimeMs::new(1_000), &resident())
        .unwrap();
    let token = ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat();
    write(tmp.path(), "work/leak.env", &format!("KEY={token}"));
    drop(first);
    let mut second = Checkpoint::open(tmp.path()).unwrap();
    let outcome = second.wave_pre(&["work".to_owned()], TimeMs::new(2_000), &resident());
    let err = match outcome {
        Err(err) => err,
        Ok(_) => panic!("the restart lost the scan of what changed before it"),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("work/leak.env"), "{rendered}");
    assert!(!rendered.contains(&token), "positions only: {rendered}");
}
