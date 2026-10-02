// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{EventKind, RunId};
use wire::{DocumentVersion, VersionSource, VersionsAnswer};

use super::*;
use crate::views::tests::{Place, view_record};

const MEMO: &str = "lab/Memo.md";

fn memo() -> Address {
    Address::parse(MEMO).unwrap()
}

fn write(dir: &Path, text: &str) {
    std::fs::create_dir_all(dir.join("lab")).unwrap();
    std::fs::write(dir.join(MEMO), text).unwrap();
}

fn store(dir: &Path) -> storage::Cas {
    storage::Cas::open(&kernel::layout::CityLayout::new(dir).cas()).unwrap()
}

/// The `document_written` line a page's save leaves.
fn saved(seq: u64, baseline: &str, version: &str) -> EventRecord {
    let data = serde_json::json!({
        "at": MEMO,
        "baseline": B3Hash::digest(baseline.as_bytes()),
        "version": B3Hash::digest(version.as_bytes()),
        "bytes": version.len(),
    });
    let serde_json::Value::Object(data) = data else {
        panic!("an object");
    };
    view_record(
        Place {
            seq,
            run: RunId::CITY,
        },
        EventKind::DocumentWritten,
        &memo(),
        data,
    )
}

fn ask(views: &mut Views) -> VersionsAnswer {
    let wire::Answer::Versions(answer) = views.answer(&wire::Query::Versions { at: memo() }) else {
        panic!("Versions answers with versions");
    };
    *answer
}

fn version(text: &str, kept: bool, source: VersionSource) -> DocumentVersion {
    DocumentVersion {
        version: B3Hash::digest(text.as_bytes()),
        bytes: kept.then(|| u64::try_from(text.len()).unwrap()),
        kept,
        source,
    }
}

/// Two saves, with a write outside the page before the second and
/// another after it: the disk first, then each save followed by the
/// version it was made on when no earlier save wrote that one. The
/// version written outside and never read is named but not kept.
#[test]
fn saves_and_the_writes_outside_the_page_are_listed_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    write(dir.path(), "zero");
    let wire::Answer::Document(_) = views.answer(&wire::Query::Document { at: memo() }) else {
        panic!("Document answers with a document");
    };
    store(dir.path()).put(b"one").unwrap();
    views.apply(&saved(1, "zero", "one")).unwrap();
    store(dir.path()).put(b"three").unwrap();
    views.apply(&saved(2, "two", "three")).unwrap();
    write(dir.path(), "four");
    let saved_at = |seq| VersionSource::Saved {
        seq: kernel::Seq::new(seq),
        at: kernel::TimeMs::new(1_000),
    };
    let before = |seq| VersionSource::Before {
        seq: kernel::Seq::new(seq),
    };
    assert_eq!(
        ask(&mut views),
        VersionsAnswer {
            at: memo(),
            versions: vec![
                version("four", true, VersionSource::OnDisk),
                version("three", true, saved_at(2)),
                version("two", false, before(2)),
                version("one", true, saved_at(1)),
                version("zero", true, before(1)),
            ],
            more: false,
        }
    );
}

/// A document nobody saved through a page has one version: the one on
/// disk, kept by being read.
#[test]
fn a_document_with_no_saves_answers_the_version_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    write(dir.path(), "only");
    assert_eq!(
        ask(&mut views).versions,
        vec![version("only", true, VersionSource::OnDisk)]
    );
}
