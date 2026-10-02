// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window of a stored document version (`crates/accounting/spec/Views/Document.lean` §8-21,
//! `crates/wire/Spec.lean` §8-70).
//!
//! Reads the content store and nothing else: the version is the
//! object's address, so a page reading on through a file a resident is
//! rewriting keeps reading the version it opened. Only the bytes the
//! window needs are lifted - its length and its first three bytes, then
//! the stretch `documents::lift` names - so the third screen of a large
//! file costs the third screen.

use std::path::Path;

use documents::{Encoding, Lifted, Span, Window};
use kernel::{AxError, B3Hash};
use storage::{Cas, StorageError};

use super::super::holding::Views;
use super::super::prepared::{Prepared, unavailable};

impl Views {
    /// A range is read from the store after the views are released.
    pub(in crate::views) fn range_ask(&self, version: B3Hash, range: Span) -> Prepared {
        Prepared::Range {
            city_root: self.city_root.clone(),
            version,
            range,
        }
    }
}

/// The window of `version` that `range` asks for, or `Unavailable`
/// when the store does not hold that version or its bytes are not text.
pub(in crate::views) fn range_answer(
    city_root: &Path,
    version: B3Hash,
    range: Span,
) -> wire::Answer {
    match window_of(city_root, &version, range) {
        Ok(window) => wire::Answer::Range(Box::new(wire::RangeAnswer { version, window })),
        // "I could not look" is the answer for every way this fails: a
        // version the store never kept, a store that will not open, bytes
        // that are not text. The query handed back names which version.
        Err(_) => unavailable(format!("Range({version})")),
    }
}

fn window_of(city_root: &Path, version: &B3Hash, wanted: Span) -> Result<Window, AxError> {
    let stored = stored(city_root, version, wanted)?;
    documents::cut(stored.encoding, stored.lifted(), wanted)
}

/// The bytes of one stored version a window needs, and the encoding its
/// mark names: the one read of the store a range and a preview both make
/// (`crates/accounting/spec/Views/Document.lean` §8-21, accounting D41).
pub(super) struct Stored {
    pub(super) encoding: Encoding,
    at: u64,
    bytes: Vec<u8>,
    size: u64,
}

impl Stored {
    pub(super) fn lifted(&self) -> Lifted<'_> {
        Lifted {
            at: self.at,
            bytes: &self.bytes,
            size: self.size,
        }
    }
}

/// Reads `version`'s length and first three bytes, then the stretch
/// `documents::lift` names for a window asked for as `wanted`.
///
/// # Errors
/// The store will not open, does not hold `version`, or will not read
/// the range.
pub(super) fn stored(city_root: &Path, version: &B3Hash, wanted: Span) -> Result<Stored, AxError> {
    let store = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(StorageError::into_ax)?;
    let size = store.size(version).map_err(StorageError::into_ax)?;
    let mark = read(&store, version, Span::new(0, size.min(3))?)?;
    let lift = documents::lift(wanted, size);
    Ok(Stored {
        encoding: Encoding::of_mark(&mark),
        at: lift.start(),
        bytes: read(&store, version, lift)?,
        size,
    })
}

/// The bytes of one half-open span of a stored object.
fn read(store: &Cas, version: &B3Hash, span: Span) -> Result<Vec<u8>, AxError> {
    if span.is_empty() {
        return Ok(Vec::new());
    }
    let closed = kernel::Range::bytes(span.start(), span.end().saturating_sub(1))?;
    store
        .get_range(version, &closed)
        .map_err(StorageError::into_ax)
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
    use kernel::Address;
    use wire::{DocumentBody, DocumentState};

    use super::*;
    use crate::views::Views;

    fn open(views: &mut Views, at: &str) -> (B3Hash, Window) {
        let at = Address::parse(at).unwrap();
        let wire::Answer::Document(answer) = views.answer(&wire::Query::Document { at }) else {
            panic!("Document answers with a document");
        };
        let DocumentState::Held(held) = answer.state else {
            panic!("{answer:?}");
        };
        let DocumentBody::Text { head, .. } = held.body else {
            panic!("the fixture is text");
        };
        (held.version, head)
    }

    fn range(views: &mut Views, version: B3Hash, start: u64, end: u64) -> wire::Answer {
        views.answer(&wire::Query::Range {
            version,
            range: Span::new(start, end).unwrap(),
        })
    }

    fn write(dir: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(dir.join("lab")).unwrap();
        std::fs::write(dir.join("lab/notes.txt"), bytes).unwrap();
    }

    /// A page reads on from the head, one window at a time, and the
    /// windows laid end to end are the file, byte for byte.
    #[test]
    fn reading_on_from_the_head_gives_back_every_byte() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = "\u{feff}行尾双空格  \r\n接续行。\n"
            .repeat(9_000)
            .into_bytes();
        write(dir.path(), &bytes);
        let mut views = Views::new(dir.path());
        let (version, head) = open(&mut views, "lab/notes.txt");
        let size = u64::try_from(bytes.len()).unwrap();
        let mut text = head.text;
        let mut at = head.span.end();
        while at < size {
            let wire::Answer::Range(answer) = range(&mut views, version, at, size) else {
                panic!("the version was kept");
            };
            assert_eq!(answer.window.span.start(), at);
            text.push_str(&answer.window.text);
            at = answer.window.span.end();
        }
        assert_eq!(text.as_bytes(), bytes.as_slice());
    }

    /// The file moves after the page opened it; the page goes on reading
    /// the version it opened.
    #[test]
    fn an_opened_version_reads_on_after_the_file_moves() {
        let dir = tempfile::tempdir().unwrap();
        let first = "a".repeat(100_000).into_bytes();
        write(dir.path(), &first);
        let mut views = Views::new(dir.path());
        let (version, _) = open(&mut views, "lab/notes.txt");
        write(dir.path(), &"b".repeat(100_000).into_bytes());
        let wire::Answer::Range(answer) = range(&mut views, version, 99_990, 100_000) else {
            panic!("the opened version was kept");
        };
        assert_eq!(answer.window.text, "a".repeat(10));
    }

    #[test]
    fn a_version_the_store_never_kept_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let never = B3Hash::digest(b"never kept");
        assert_eq!(
            range(&mut views, never, 0, 10),
            wire::Answer::Unavailable {
                query: format!("Range({never})"),
            }
        );
    }
}
