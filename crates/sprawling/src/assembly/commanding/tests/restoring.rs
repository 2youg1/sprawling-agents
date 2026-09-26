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

use crate::assembly::*;

#[test]
fn a_discarded_file_comes_back_and_its_row_closes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_city(root).unwrap();
    let mut worker = RunWorker::new(
        root,
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();

    let doomed = root.join("lab/room1/notes.md");
    std::fs::create_dir_all(doomed.parent().unwrap()).unwrap();
    std::fs::write(&doomed, "the words a wave deleted").unwrap();
    let of = memory::Provenance::new(
        RunId::parse("018f5b2a-0000-7000-8000-000000000001").unwrap(),
        Address::parse("lab/room1").unwrap(),
        kernel::B3Hash::digest(b"a city"),
        memory::ModelChoice {
            id: "test-model".to_owned(),
            effort: None,
        },
    );
    let mut fence = memory::Checkpoint::open(root).unwrap();
    let pre = fence
        .wave_pre(&["lab".to_owned()], kernel::TimeMs::new(1_000), &of)
        .unwrap();
    let pre_oid = serde_json::to_value(&pre).unwrap()["oid"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::remove_file(&doomed).unwrap();
    let swept = fence.wave_post(&pre_oid).unwrap();
    let restoration: kernel::Restoration =
        serde_json::from_value(serde_json::to_value(&swept[0]).unwrap()["restoration"].clone())
            .unwrap();
    worker
        .record(EventKind::FileDiscarded, swept[0].clone())
        .unwrap();

    worker
        .handle(channels::Command::RestoreDiscard {
            restoration,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"restore"),
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&doomed).ok().as_deref(),
        Some("the words a wave deleted"),
        "the bytes the fence held are back at their path"
    );
    let channels::Answer::Discards(bin) =
        crate::views::ask(root, &channels::Query::DiscardView).unwrap()
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
