// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file of the city, read once from disk and answered as a version
//! (accounting-SPEC.md 8-21, wire-SPEC.md 8-69).
//!
//! Every judgement - which version, whether the bytes are text, where
//! the first window ends - is the `documents` crate's; this module reads
//! the disk, keeps a version in the content store when its first window
//! does not cover it, and spells the answer. The same text judgement
//! answers `Content` and `Prefix`, which read bytes out of the store
//! rather than off the tree: what makes bytes readable does not depend on
//! where they were kept.

use std::path::Path;

use documents::{Format, Lifted, Reading as Judged, Span};
use kernel::{Address, AxError, B3Hash};
use wire::{Coverage, DocumentBody, DocumentState, HeldDocument};

use super::listing::resolve;

/// One file of the tree as it stands at the moment of asking.
///
/// Takes the city root rather than the views: it reads the disk, and
/// runs after the view lock is released (sprawling-SPEC.md 8-100).
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

/// What one file's bytes are, as an answer.
fn state_of(city_root: &Path, at: &Address, bytes: &[u8]) -> DocumentState {
    let version = B3Hash::digest(bytes);
    let format = Format::of_name(at.as_str());
    if bytes.is_empty() {
        return DocumentState::Empty { version, format };
    }
    let body = match Judged::of(bytes) {
        Judged::Opaque => DocumentBody::Opaque,
        Judged::Text(encoding) => match text_body(city_root, format, encoding, bytes) {
            Ok(body) => body,
            // A version whose head cannot be offered is not one a page
            // can read: the store would not keep it, so a `Head` answer
            // would promise a range read that cannot happen.
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

/// The first window of a text version, and the version kept in the
/// store when that window does not cover it (accounting-SPEC.md 8-21).
fn text_body(
    city_root: &Path,
    format: Format,
    encoding: documents::Encoding,
    bytes: &[u8],
) -> Result<DocumentBody, AxError> {
    let head = documents::head(format, encoding, bytes)?;
    let coverage = if head.span.end() < length(bytes) {
        keep(city_root, bytes)?;
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

/// Puts one version in the city's content store, where `Query::Range`
/// reads it by the address that is also its version.
fn keep(city_root: &Path, bytes: &[u8]) -> Result<(), AxError> {
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

/// Bytes as `Content` and `Prefix` give them: the first window, with
/// the cut and the text judgement stated.
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
        // Bytes judged text always cut at a boundary; were the rules ever
        // to disagree with themselves, what nobody can read is shown as
        // nothing rather than as noise.
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

    /// The rules that govern a building are readable through the tree,
    /// whole, as the version on disk.
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

    /// Nothing at the address, something there that will not read, and
    /// an empty file are three answers, not one refusal.
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

    /// A UTF-16 file holds a NUL beside every ASCII character; its mark
    /// says it is text, and it is read as text.
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

    /// A file longer than one window answers its head, and the version
    /// it read is in the store, where a range read finds it.
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

    /// `Content` and `Prefix` keep their shape and take the same text
    /// judgement: a NUL without a mark is not text, a long text is cut.
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
