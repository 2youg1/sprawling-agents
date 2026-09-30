// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use crate::worker::*;

#[test]
fn a_discarded_file_comes_back_and_its_row_closes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    crate::worker::fixture::init_city(root).unwrap();
    let mut worker = RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();

    let doomed = root.join("lab/room1/notes.md");
    std::fs::create_dir_all(doomed.parent().unwrap()).unwrap();
    std::fs::write(&doomed, "the words a wave deleted").unwrap();
    let of = storage::Provenance::new(
        RunId::parse("018f5b2a-0000-7000-8000-000000000001").unwrap(),
        Address::parse("lab/room1").unwrap(),
        kernel::B3Hash::digest(b"a city"),
        storage::ModelChoice {
            id: "test-model".to_owned(),
            effort: None,
        },
    );
    let mut checkpoint = storage::Checkpoint::open(root).unwrap();
    let pre = checkpoint
        .wave_pre(&["lab".to_owned()], kernel::TimeMs::new(1_000), &of)
        .unwrap();
    let pre_oid = serde_json::to_value(&pre).unwrap()["oid"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::remove_file(&doomed).unwrap();
    let swept = checkpoint.wave_post(&pre_oid).unwrap();
    let restoration: kernel::Restoration =
        serde_json::from_value(serde_json::to_value(&swept[0]).unwrap()["restoration"].clone())
            .unwrap();
    worker
        .record(EventKind::FileDiscarded, swept[0].clone())
        .unwrap();

    worker
        .handle(wire::Command::RestoreDiscard {
            restoration,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"restore"),
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&doomed).ok().as_deref(),
        Some("the words a wave deleted"),
        "the bytes the checkpoint held are back at their path"
    );
    let wire::Answer::Discards(bin) = crate::views::ask(root, &wire::Query::DiscardView).unwrap()
    else {
        panic!("DiscardView answers with the bin");
    };
    assert_eq!(
        bin.rows
            .iter()
            .map(|row| (row.path.as_str(), row.restored))
            .collect::<Vec<_>>(),
        vec![("file:lab/room1/notes.md", true)],
        "the restoration closes the row the discard opened"
    );
}

#[test]
fn a_way_back_the_bin_does_not_write_is_refused_in_one_readable_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    crate::worker::fixture::init_city(root).unwrap();
    let mut worker = RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    let stored = kernel::Locator::cas(kernel::B3Hash::digest(b"a stored object"));

    let refusal = worker
        .handle(wire::Command::RestoreDiscard {
            restoration: kernel::Restoration::Tracked(stored.clone()),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"restore"),
        })
        .unwrap_err();

    assert_eq!(
        refusal,
        kernel::AxError::failure(
            kernel::AxCode::InvalidArgs,
            "restore a discarded file",
            stored.to_string(),
        )
        .with_recovery(
            "restore the whole file: a way back that names a part of one, or no file, is not one \
             the recycle bin writes"
        ),
        "the person reads the recovery as one sentence, with no run of blanks inside it"
    );
}
