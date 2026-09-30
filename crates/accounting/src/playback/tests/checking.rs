// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A bundle checked on its own, against another, and against its city;
//! and the bytes that let it sit inside an HTML script block.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{EventKind, RunId};
use serde_json::json;

use super::super::document::Decimal;
use super::super::{Against, Confidential, Reader, Verdict, check, encode, export};
use super::{city, lines, parsed, person, run, write};

#[test]
fn a_changed_bundle_that_keeps_its_source_differs_from_its_city() {
    let (dir, _) = city();
    let exported = export(dir.path(), &person()).unwrap();
    let (mut document, _) = encode::decode(exported.bytes()).unwrap();
    document.costs.billed_usd_micros = Decimal(999);
    let changed = encode::encode(&document).unwrap();
    let as_the_person = || Against::City {
        root: dir.path(),
        reader: Reader::Person(Confidential::Withheld),
    };
    let verdict = |bytes: &[u8], against| check(bytes, against).map(|report| report.verdict);
    assert_eq!(
        (
            verdict(changed.bytes(), Against::Nothing),
            verdict(changed.bytes(), as_the_person()),
            verdict(changed.bytes(), Against::Bundle(exported.bytes())),
            verdict(exported.bytes(), as_the_person()),
        ),
        (
            Ok(Verdict::Consistent),
            Ok(Verdict::Differs { section: "costs" }),
            Ok(Verdict::Differs { section: "costs" }),
            Ok(Verdict::Same),
        )
    );
}

#[test]
fn a_bundle_is_not_recomputed_for_a_reader_it_was_not_exported_for() {
    let (dir, _) = city();
    let exported = export(dir.path(), &person()).unwrap();
    let widened = check(
        exported.bytes(),
        Against::City {
            root: dir.path(),
            reader: Reader::Person(Confidential::Included),
        },
    )
    .map(|report| matches!(report.verdict, Verdict::CannotReproduce { .. }));
    assert_eq!(widened, Ok(true));
}

#[test]
fn a_script_end_tag_in_a_line_cannot_close_an_html_block() {
    let dir = tempfile::tempdir().unwrap();
    let written = lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            run(1),
            Some("lab/a"),
            EventKind::ToolCalled,
            json!({"tool": "</script><b>&"}),
        ),
    ]);
    write(dir.path(), &written, b"");
    let exported = export(dir.path(), &person()).unwrap();
    let text = String::from_utf8(exported.bytes().to_vec()).unwrap();
    let back = parsed(exported.bytes());
    assert_eq!(
        (
            text.contains('<') || text.contains('>') || text.contains('&'),
            back["events"][1]["line"].clone()
        ),
        (false, json!(String::from_utf8(written[1].clone()).unwrap()))
    );
}
