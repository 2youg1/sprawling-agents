// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use documents::{Format, Lifted, Reading as Judged, Span};
use kernel::{Address, AxError, B3Hash};
use wire::{Coverage, DocumentBody, DocumentState, HeldDocument};

use super::holding::Views;
use super::listing::resolve;
use super::prepared::Prepared;

impl Views {
    pub(in crate::views) fn document_ask(&self, at: &Address) -> Prepared {
        Prepared::Document {
            city_root: self.city_root.clone(),
            at: at.clone(),
        }
    }
}

pub(super) fn document_answer(city_root: &Path, at: Address) -> wire::DocumentAnswer {
    let state = match std::fs::read(resolve(city_root, Some(&at))) {
        Ok(bytes) => state_of(city_root, &at, &bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => DocumentState::Missing,
        Err(err) => DocumentState::Unreadable {
            reason: err.to_string(),
        },
    };
    wire::DocumentAnswer { at, state }
}

fn state_of(city_root: &Path, at: &Address, bytes: &[u8]) -> DocumentState {
    let version = B3Hash::digest(bytes);
    let format = Format::of_name(at.as_str());

    if let Err(err) = keep(city_root, bytes) {
        return DocumentState::Unreadable {
            reason: err.to_string(),
        };
    }
    if bytes.is_empty() {
        return DocumentState::Empty { version, format };
    }
    let body = match Judged::of(bytes) {
        Judged::Opaque => DocumentBody::Opaque,
        Judged::Text(encoding) => match text_body(format, encoding, bytes) {
            Ok(body) => body,

            Err(err) => {
                return DocumentState::Unreadable {
                    reason: err.to_string(),
                };
            }
        },
    };
    DocumentState::Held(Box::new(HeldDocument {
        version,
        format,
        bytes: length(bytes),
        body,
    }))
}

fn text_body(
    format: Format,
    encoding: documents::Encoding,
    bytes: &[u8],
) -> Result<DocumentBody, AxError> {
    let head = documents::head(format, encoding, bytes)?;
    let coverage = if head.span.end() < length(bytes) {
        Coverage::Head
    } else {
        Coverage::Whole
    };
    Ok(DocumentBody::Text {
        encoding,
        head,
        coverage,
    })
}

pub(crate) fn keep(city_root: &Path, bytes: &[u8]) -> Result<(), AxError> {
    let mut store = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(storage::StorageError::into_ax)?;
    store
        .put(bytes)
        .map(drop)
        .map_err(storage::StorageError::into_ax)
}

fn length(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

pub(super) struct Reading {
    pub(super) text: String,
    pub(super) bytes: u64,
    pub(super) truncated: bool,
    pub(super) binary: bool,
}

pub(super) fn read_bytes(bytes: &[u8]) -> Reading {
    let size = length(bytes);
    let opaque = Reading {
        text: String::new(),
        bytes: size,
        truncated: false,
        binary: true,
    };
    let Judged::Text(encoding) = Judged::of(bytes) else {
        return opaque;
    };
    let lifted = Lifted { at: 0, bytes, size };
    match Span::new(0, size).and_then(|whole| documents::cut(encoding, lifted, whole)) {
        Ok(window) => Reading {
            truncated: window.span.end() < size,
            text: window.text,
            bytes: size,
            binary: false,
        },

        Err(_) => opaque,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::views::Views;

    fn ask(dir: &Path, at: &str) -> wire::DocumentAnswer {
        let mut views = Views::new(dir);
        let at = Address::parse(at).unwrap();
        let wire::Answer::Document(answer) = views.answer(&wire::Query::Document { at }) else {
            panic!("Document answers with a document");
        };
        *answer
    }

    fn held(answer: wire::DocumentAnswer) -> HeldDocument {
        let DocumentState::Held(held) = answer.state else {
            panic!("{answer:?}");
        };
        *held
    }

    fn utf16_le(text: &str) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    }

    #[test]
    fn a_governing_file_reads_back_whole_as_its_version() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let at = format!("hall/{}/{}", kernel::RESERVED_PREFIX, city::RULES_FILE);
        let on_disk = std::fs::read(dir.path().join(&at)).unwrap();
        let held = held(ask(dir.path(), &at));
        assert_eq!(held.version, B3Hash::digest(&on_disk));
        assert_eq!(held.bytes, length(&on_disk));
        let DocumentBody::Text { head, coverage, .. } = held.body else {
            panic!("RULES.toml is text");
        };
        assert_eq!(coverage, Coverage::Whole);
        assert_eq!(head.text.as_bytes(), on_disk.as_slice());
    }

    #[test]
    fn missing_unreadable_and_empty_are_three_answers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hall/room")).unwrap();
        std::fs::write(dir.path().join("hall/Empty.md"), b"").unwrap();
        assert_eq!(
            ask(dir.path(), "hall/nothing.md").state,
            DocumentState::Missing
        );
        assert!(
            matches!(
                ask(dir.path(), "hall/room").state,
                DocumentState::Unreadable { .. }
            ),
            "a directory does not read as a file"
        );
        assert_eq!(
            ask(dir.path(), "hall/Empty.md").state,
            DocumentState::Empty {
                version: B3Hash::digest(b""),
                format: Format::Markdown,
            }
        );
    }

    #[test]
    fn a_utf16_file_is_text_whatever_nul_it_holds() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = utf16_le("notes\r\n段落\r\n");
        std::fs::create_dir_all(dir.path().join("hall")).unwrap();
        std::fs::write(dir.path().join("hall/notes.txt"), &bytes).unwrap();
        let held = held(ask(dir.path(), "hall/notes.txt"));
        assert_eq!(
            held.body,
            DocumentBody::Text {
                encoding: documents::Encoding::Utf16Le,
                head: documents::Window {
                    span: Span::new(0, length(&bytes)).unwrap(),
                    text: "\u{feff}notes\r\n段落\r\n".to_owned(),
                },
                coverage: Coverage::Whole,
            }
        );
    }

    #[test]
    fn bytes_with_a_nul_and_no_mark_are_opaque() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("x/views")).unwrap();
        std::fs::write(dir.path().join("x/views/db.redb"), b"redb\0\x01\x02").unwrap();
        let held = held(ask(dir.path(), "x/views/db.redb"));
        assert_eq!(held.body, DocumentBody::Opaque);
        assert_eq!(held.bytes, 7);
    }

    #[test]
    fn a_long_file_answers_its_head_and_keeps_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let long = "一段话。\n\n".repeat(20_000).into_bytes();
        std::fs::create_dir_all(dir.path().join("lab")).unwrap();
        std::fs::write(dir.path().join("lab/Memo.md"), &long).unwrap();
        let held = held(ask(dir.path(), "lab/Memo.md"));
        let DocumentBody::Text { head, coverage, .. } = held.body else {
            panic!("Memo.md is text");
        };
        assert_eq!(coverage, Coverage::Head);
        assert!(head.span.end() <= documents::WINDOW_BYTES_MAX);
        let store = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas()).unwrap();
        assert!(store.contains(&held.version));
        assert_eq!(held.version, B3Hash::digest(&long));
    }

    #[test]
    fn every_version_a_read_sees_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("lab")).unwrap();
        std::fs::write(dir.path().join("lab/notes.txt"), b"short\n").unwrap();
        std::fs::write(dir.path().join("lab/scan.pdf"), b"%PDF\0\x01").unwrap();
        let notes = held(ask(dir.path(), "lab/notes.txt"));
        let scan = held(ask(dir.path(), "lab/scan.pdf"));
        assert_eq!(scan.body, DocumentBody::Opaque);
        let store = storage::Cas::open(&kernel::layout::CityLayout::new(dir.path()).cas()).unwrap();
        assert!(store.contains(&notes.version), "a small whole text is kept");
        assert!(store.contains(&scan.version), "an opaque version is kept");
    }

    #[test]
    fn stored_bytes_are_judged_and_cut_by_the_document_rules() {
        let opaque = read_bytes(b"redb\0\x01\x02");
        assert!(opaque.binary);
        assert_eq!((opaque.text.as_str(), opaque.bytes), ("", 7));
        let long = vec![b'a'; 70_000];
        let cut = read_bytes(&long);
        assert!(cut.truncated && !cut.binary);
        assert_eq!(length(cut.text.as_bytes()), documents::WINDOW_BYTES_MAX);
        let marked = read_bytes(&utf16_le("a"));
        assert_eq!(marked.text, "\u{feff}a");
    }
}
