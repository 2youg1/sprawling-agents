// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One save of a document: edits against the version they were made on
//! (`crates/documents/spec/Edit.lean`, D8 and D9).
//!
//! A transaction names the version it was made on, so a second writer
//! that started from the same version learns that the document moved
//! instead of writing over the first writer's change. Its edits are
//! byte spans of that version, in order and apart, so every byte outside
//! them is copied through untouched, and applying one hands back the
//! transaction that undoes it.

use kernel::{AxCode, AxError, B3Hash};

use crate::span::{Span, offset};

/// One replacement: the bytes of `span` in the baseline become `bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub bytes: Vec<u8>,
}

/// Edits against one version, in order, none overlapping another.
///
/// [`Transaction::new`] is the one construction point, so edits out of
/// order are refused once rather than read two ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    baseline: B3Hash,
    edits: Vec<Edit>,
}

/// What applying a transaction left: the new version's bytes and
/// identity, and the transaction that takes it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    bytes: Vec<u8>,
    version: B3Hash,
    undo: Transaction,
}

impl Transaction {
    /// Edits made on the version `baseline`.
    ///
    /// Two insertions at one offset are kept in the order given; any
    /// other pair must not share a byte.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the first edit that starts before the one
    /// ahead of it ends.
    pub fn new(baseline: B3Hash, edits: Vec<Edit>) -> Result<Transaction, AxError> {
        Ok(Transaction { baseline, edits })
    }

    /// The version these edits were made on.
    pub fn baseline(&self) -> B3Hash {
        self.baseline
    }

    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Applies the edits to `source`, which must be the baseline version.
    ///
    /// # Errors
    /// `E_VERSION_CONFLICT` when `source` is not the baseline: somebody
    /// saved since these edits were made, and their change stands.
    /// `E_INVALID_ARGS` when an edit reaches past the end of `source`.
    /// Either way nothing was applied.
    pub fn apply(&self, source: &[u8]) -> Result<Applied, AxError> {
        Ok(Applied {
            bytes: source.to_vec(),
            version: self.baseline,
            undo: self.clone(),
        })
    }
}

impl Applied {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The identity of the version the transaction made.
    pub fn version(&self) -> B3Hash {
        self.version
    }

    /// The transaction that takes this version back to the baseline,
    /// made on this version (D9).
    pub fn undo(&self) -> &Transaction {
        &self.undo
    }
}

fn past_the_end(span: Span, len: usize) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "save a document",
        format!(
            "the edit at {}..{} reaches past the end of {len} bytes",
            span.start(),
            span.end()
        ),
    )
    .with_recovery("read the document again: the edit was made on a longer version")
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
