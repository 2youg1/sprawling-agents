// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A person's steps on the prompt documents of one building, through
//! the worker's own doors, read back through the layers answer
//! (`crates/wire/Spec.lean` §8-98; `crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::B3Hash;

use crate::worker::*;

const OWN: &str = "lab/.sprawling/City.md";
const CITY: &str = "City.md";

fn at(spelled: &str) -> Address {
    Address::parse(spelled).unwrap()
}

fn key(name: &[u8]) -> kernel::IdemKey {
    kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, name)
}

/// A city with the building `lab`, whose `City.md` at the root says one
/// line.
fn city() -> (tempfile::TempDir, RunWorker) {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: at("lab"),
            template: wire::TemplateName::parse("minimal").unwrap(),
            rules: None,
            idem: key(b"lab"),
        })
        .unwrap();
    std::fs::write(dir.path().join(CITY), "the city's words\n").unwrap();
    (dir, worker)
}

fn put_prompt(doc: &str, base: &[u8], step: wire::PromptStep, idem: &[u8]) -> wire::Command {
    wire::Command::PutPrompt {
        at: at(doc),
        base: B3Hash::digest(base),
        step,
        idem: key(idem),
    }
}

/// What the layers answer says of the two `City.md` of `lab`, the
/// city's first.
fn city_slot(root: &Path) -> Vec<(Address, wire::LayerState)> {
    let wire::Answer::Layers(answer) =
        crate::views::ask(root, &wire::Query::Layers { addr: at("lab") }).unwrap()
    else {
        panic!("Layers answers with layers");
    };
    answer
        .documents
        .into_iter()
        .filter(|doc| doc.kind == wire::PromptKind::City)
        .map(|doc| (doc.at, doc.state))
        .collect()
}

/// The payloads of every line of one kind, in the order written.
fn lines(root: &Path, kind: &str) -> Vec<serde_json::Value> {
    let ledger = kernel::layout::CityLayout::new(root).ledger();
    runtime::replay::verify_ledger_dir(&ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|value| value["kind"] == kind)
        .map(|value| value["data"].clone())
        .collect()
}

/// A building starts its own `City.md` from the city's words, and a
/// page then saves it word by word through `PutRange`, which admits it
/// although it sits in the building's reserved subtree. The building's
/// copy then governs, and the city's is named as replaced by it.
#[test]
fn a_page_saves_a_buildings_own_city_md_word_by_word() {
    let (dir, mut worker) = city();
    worker
        .handle(put_prompt(OWN, b"", wire::PromptStep::Own, b"own"))
        .unwrap();
    worker
        .handle(wire::Command::PutRange(wire::RangeWrite {
            doc: at(OWN),
            baseline: B3Hash::digest(b"the city's words\n"),
            edits: vec![documents::TextEdit {
                span: documents::Span::new(0, 0).unwrap(),
                text: "Lab rule.\n".to_owned(),
            }],
            idem: key(b"save"),
        }))
        .unwrap();

    assert_eq!(
        (
            std::fs::read_to_string(dir.path().join(OWN)).unwrap(),
            lines(dir.path(), "document_written")
                .last()
                .map(|line| line["at"].clone()),
            city_slot(dir.path()),
        ),
        (
            "Lab rule.\nthe city's words\n".to_owned(),
            Some(serde_json::json!(OWN)),
            vec![
                (at(CITY), wire::LayerState::Replaced { by: at(OWN) }),
                (at(OWN), wire::LayerState::Sent { bytes: 27 }),
            ],
        )
    );
}

/// Taking the building's copy away gives the building back to the
/// city's `City.md`, and the line says the file was removed rather than
/// written empty.
#[test]
fn inheriting_gives_the_building_back_to_the_city_md() {
    let (dir, mut worker) = city();
    worker
        .handle(put_prompt(OWN, b"", wire::PromptStep::Own, b"own"))
        .unwrap();
    worker
        .handle(put_prompt(
            OWN,
            b"the city's words\n",
            wire::PromptStep::Inherit,
            b"inherit",
        ))
        .unwrap();

    assert_eq!(
        (
            city_slot(dir.path()),
            lines(dir.path(), "governed_document_written")
                .last()
                .cloned(),
        ),
        (
            vec![
                (at(CITY), wire::LayerState::Sent { bytes: 17 }),
                (at(OWN), wire::LayerState::Absent),
            ],
            Some(serde_json::json!({
                "which": "City.md",
                "bytes": 0,
                "at": OWN,
                "removed": true,
            })),
        )
    );
}

/// A save that would leave the Mayor's identity area unreadable is
/// refused before anything lands, as the settings page's door refuses it.
#[test]
fn a_word_by_word_save_cannot_break_the_mayors_names() {
    let (dir, mut worker) = city();
    let mayor = dir.path().join(".sprawling/MAYOR.md");
    let before = std::fs::read(&mayor).unwrap();

    let refused = worker
        .handle(wire::Command::PutRange(wire::RangeWrite {
            doc: at(".sprawling/MAYOR.md"),
            baseline: B3Hash::digest(&before),
            edits: vec![documents::TextEdit {
                span: documents::Span::new(0, 0).unwrap(),
                text: "+++\nname = [\n+++\n".to_owned(),
            }],
            idem: key(b"break"),
        }))
        .unwrap_err();

    assert_eq!(
        (refused.code(), std::fs::read(&mayor).unwrap()),
        (&kernel::AxCode::ConfigInvalid, before),
        "{refused}"
    );
}
