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

use documents::Span;
use kernel::{Address, B3Hash};

use super::super::holding::Views;
use super::super::prepared::{Prepared, unavailable};

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
        let _ = &self.city_root;
        match self.read {
            StoredRead::Bytes(_) => unavailable(format!("Bytes({})", self.version)),
            StoredRead::Export(_) => unavailable(format!("Export({})", self.version)),
        }
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
mod tests;
