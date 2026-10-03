// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use base64::Engine;
use wire::{DocumentState, HeldDocument};

use super::*;
use crate::views::Views;

fn held(views: &mut Views, at: &Address) -> HeldDocument {
    let wire::Answer::Document(answer) = views.answer(&wire::Query::Document { at: at.clone() })
    else {
        panic!("Document answers with a document");
    };
    let DocumentState::Held(held) = answer.state else {
        panic!("{answer:?}");
    };
    *held
}

fn bytes(views: &mut Views, version: B3Hash, start: u64) -> wire::Answer {
    views.answer(&wire::Query::Bytes {
        version,
        range: Span::new(start, u64::MAX).unwrap(),
    })
}

/// A PDF written into a building comes back through the door window by
/// window, and the windows laid end to end are the file, byte for byte.
#[test]
fn a_pdf_written_to_a_building_reads_back_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    pdf.extend((0..1_500_000_u32).map(|n| u8::try_from(n % 251).unwrap()));
    pdf.extend_from_slice(b"\n%%EOF\n");
    std::fs::create_dir_all(dir.path().join("lab/out")).unwrap();
    std::fs::write(dir.path().join("lab/out/report.pdf"), &pdf).unwrap();
    let mut views = Views::new(dir.path());
    let at = Address::parse("lab/out/report.pdf").unwrap();
    let version = held(&mut views, &at).version;
    let mut read = Vec::new();
    let mut windows = 0;
    loop {
        let wire::Answer::Bytes(answer) =
            bytes(&mut views, version, u64::try_from(read.len()).unwrap())
        else {
            panic!("the version read is kept");
        };
        assert_eq!(answer.size, u64::try_from(pdf.len()).unwrap());
        assert_eq!(answer.span.start(), u64::try_from(read.len()).unwrap());
        assert!(answer.span.len() <= wire::BYTES_WINDOW_MAX);
        windows += 1;
        if answer.span.is_empty() {
            break;
        }
        read.extend(
            base64::engine::general_purpose::STANDARD
                .decode(&answer.base64)
                .unwrap(),
        );
    }
    assert_eq!(read, pdf);
    assert_eq!(
        windows, 3,
        "two windows of bytes and the empty one at the end"
    );
}

/// An object the store never kept is "I could not look", by name.
#[test]
fn bytes_the_store_does_not_hold_are_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let mut views = Views::new(dir.path());
    let version = B3Hash::digest(b"never kept");
    assert_eq!(
        bytes(&mut views, version, 0),
        wire::Answer::Unavailable {
            query: format!("Bytes({version})"),
            reason: None,
        }
    );
}

/// A Markdown version read once exports as one HTML page with its
/// heading and link, titled by the document's name; the export reads
/// the version, not the file the disk holds now.
#[test]
fn a_kept_markdown_version_exports_as_one_page() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab/Memo.md"),
        "# Memo\n\nSee [the plan](https://example.org/plan).\n",
    )
    .unwrap();
    let mut views = Views::new(dir.path());
    let at = Address::parse("lab/Memo.md").unwrap();
    let version = held(&mut views, &at).version;
    std::fs::write(dir.path().join("lab/Memo.md"), "# Rewritten\n").unwrap();
    let wire::Answer::Export(answer) = views.answer(&wire::Query::Export {
        at: at.clone(),
        version,
    }) else {
        panic!("a kept Markdown version exports");
    };
    assert_eq!(answer.version, version);
    assert!(
        answer.html.contains("<title>Memo.md</title>"),
        "{}",
        answer.html
    );
    assert!(answer.html.contains("<h1>Memo</h1>"), "{}", answer.html);
    assert!(
        answer
            .html
            .contains("<a href=\"https://example.org/plan\">the plan</a>"),
        "{}",
        answer.html
    );
    let missing = B3Hash::digest(b"never kept");
    assert_eq!(
        views.answer(&wire::Query::Export {
            at,
            version: missing
        }),
        wire::Answer::Unavailable {
            query: format!("Export({missing})"),
            reason: None,
        }
    );
}
