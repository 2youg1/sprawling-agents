// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stored object as bytes, and a stored Markdown version exported
//! (`crates/accounting/spec/Views/Document.lean` §8-21, `crates/wire/spec/Answer/DocumentBytes.lean`
//! §8-80, §8-81).
//!
//! Both read the content store and nothing else, after the view lock is
//! released, the way a range does: the version is the object's address,
//! so what a page reads is the version it named, whatever the file on
//! disk has become since.

use std::path::PathBuf;

use base64::Engine as _;
use documents::{Encoding, Span};
use kernel::{Address, AxError, B3Hash};
use storage::{Cas, StorageError};

use super::super::holding::Views;
use super::super::prepared::{Prepared, unavailable};
use super::range::read;

/// A read of one stored object, still to be made.
pub struct StoredAsk {
    city_root: PathBuf,
    version: B3Hash,
    read: StoredRead,
}

/// What is read of the object.
enum StoredRead {
    /// One window of its bytes.
    Bytes(Span),
    /// The whole of it, exported as HTML titled by the document's name.
    Export(Address),
}

impl Views {
    pub(in crate::views) fn bytes_ask(&self, version: B3Hash, range: Span) -> Prepared {
        self.stored_ask(version, StoredRead::Bytes(range))
    }

    pub(in crate::views) fn export_ask(&self, at: &Address, version: B3Hash) -> Prepared {
        self.stored_ask(version, StoredRead::Export(at.clone()))
    }

    fn stored_ask(&self, version: B3Hash, read: StoredRead) -> Prepared {
        Prepared::Stored(StoredAsk {
            city_root: self.city_root.clone(),
            version,
            read,
        })
    }
}

impl StoredAsk {
    /// The answer, or `Unavailable` naming the question when the store
    /// does not hold the object or the export is refused.
    pub(in crate::views) fn answer(self) -> wire::Answer {
        let store = match Cas::open(&kernel::layout::CityLayout::new(&self.city_root).cas()) {
            Ok(store) => store,
            Err(_) => return self.unanswered(),
        };
        let answered = match &self.read {
            StoredRead::Bytes(range) => bytes_of(&store, self.version, *range)
                .map(|bytes| wire::Answer::Bytes(Box::new(bytes))),
            StoredRead::Export(at) => export_of(&store, self.version, at).map(|html| {
                wire::Answer::Export(Box::new(wire::ExportAnswer {
                    version: self.version,
                    html,
                }))
            }),
        };
        // "I could not look" is the answer for every way this fails, as
        // it is for a range: an object the store never kept, a store that
        // will not read, a version the export refuses.
        answered.unwrap_or_else(|_| self.unanswered())
    }

    fn unanswered(&self) -> wire::Answer {
        match self.read {
            StoredRead::Bytes(_) => unavailable(format!("Bytes({})", self.version)),
            StoredRead::Export(_) => unavailable(format!("Export({})", self.version)),
        }
    }
}

/// The window of the object `range` asks for: from its start, at most
/// [`wire::BYTES_WINDOW_MAX`] bytes, never past the end.
fn bytes_of(store: &Cas, version: B3Hash, range: Span) -> Result<wire::BytesAnswer, AxError> {
    let size = store.size(&version).map_err(StorageError::into_ax)?;
    let start = range.start().min(size);
    let end = range
        .end()
        .min(size)
        .min(start.saturating_add(wire::BYTES_WINDOW_MAX));
    let span = Span::new(start, end)?;
    let bytes = read(store, &version, span)?;
    Ok(wire::BytesAnswer {
        version,
        span,
        size,
        base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

/// The whole version, exported under the document's file name.
fn export_of(store: &Cas, version: B3Hash, at: &Address) -> Result<String, AxError> {
    let bytes = store.get(&version).map_err(StorageError::into_ax)?;
    let title = at.as_str().rsplit('/').next().unwrap_or(at.as_str());
    documents::export(title, Encoding::of_mark(&bytes), &bytes)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
