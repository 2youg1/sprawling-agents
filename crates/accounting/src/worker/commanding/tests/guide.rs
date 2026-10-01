// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use crate::worker::*;

/// Where the person stands in the guide is the city's, not the
/// browser's: a page opened after the worker that took the write is gone
/// reads the progress back as it was written, and a city nobody guided
/// reads as a guide at its start (accounting-SPEC.md 8-18-2).
#[test]
fn the_guide_keeps_its_progress_across_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    crate::worker::fixture::init_city(root).unwrap();
    let unguided = crate::views::ask(root, &wire::Query::Guide).unwrap();
    let progress = wire::GuideProgress {
        at: Some(wire::GuideStep::Skills),
        state: wire::GuideState::Left,
        dependencies: Some(wire::GuideMark::Skipped),
        texts: Some(wire::GuideMark::Seen),
        skills: None,
        mcp: Some(wire::GuideMark::Skipped),
    };
    let mut worker = RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::PutGuide {
            progress: progress.clone(),
            idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"guide"),
        })
        .unwrap();
    drop(worker);

    let reopened = crate::views::ask(root, &wire::Query::Guide).unwrap();

    assert_eq!(
        (unguided, reopened),
        (
            wire::Answer::Guide(wire::GuideProgress::default()),
            wire::Answer::Guide(progress),
        )
    );
}
