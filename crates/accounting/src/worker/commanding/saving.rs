// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A page's save of one document, and a person's decision on the
//! proposal cards of one (accounting-SPEC.md 8-22, wire-SPEC.md 8-72,
//! 8-73).
//!
//! Both read the document's bytes inside its lock, hand them to the
//! `documents` rule that decides, replace the file whole, and then record
//! what landed. The rules are the `documents` crate's and the lock is
//! `city::document`'s; this module only puts them in order and writes
//! the lines.

use std::path::PathBuf;

use documents::{Applied, Offer};
use kernel::event::record::{DocumentWritten, ProposalDecided, SliceVerdict};
use kernel::{Address, AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;

impl RunWorker {
    /// Saves a page's edits on the version they were made on, and
    /// records the save with the version it left.
    ///
    /// # Errors
    /// `E_OUTSIDE_WRITE_DOMAIN` for the reserved subtree; whatever
    /// `documents::save` refuses (a version that moved, edits that do not
    /// fit, a result that is not text); a file that cannot be read or
    /// replaced; and a history that will not take the line.
    pub(in crate::worker) fn put_range(&mut self, write: &wire::RangeWrite) -> Result<(), AxError> {
        let path = self.document_path(&write.doc)?;
        let applied = city::revise_document(&path, |held, source| {
            let applied = documents::save(source, write.baseline, &write.edits)?;
            held.replace(applied.bytes())?;
            Ok(applied)
        })?;
        self.document_written(&write.doc, &applied)
    }

    /// Lands what a person accepted of the proposal cards they decided
    /// as one save, then records each card as decided.
    ///
    /// # Errors
    /// `E_OUTSIDE_WRITE_DOMAIN` for the reserved subtree;
    /// `E_INVALID_ARGS` for a card that is not open on this document;
    /// whatever `documents::decide` refuses; a file that cannot be read
    /// or replaced; and a history that will not take a line. Nothing is
    /// written or recorded when the decision is refused.
    pub(in crate::worker) fn decide_proposals(
        &mut self,
        decisions: &wire::ProposalDecisions,
    ) -> Result<(), AxError> {
        self.document_path(&decisions.doc).map(drop)
    }

    /// Where a document a page names lives, refused inside the reserved
    /// subtree: the files there have doors of their own that judge what
    /// they are given before it lands (wire-SPEC.md 8-72).
    fn document_path(&self, doc: &Address) -> Result<PathBuf, AxError> {
        if doc.is_reserved() {
            return Err(AxError::failure(
                AxCode::OutsideWriteDomain,
                "save a document",
                doc.as_str().to_owned(),
            )
            .with_recovery(
                "write the city's and the buildings' rules, configuration and governing documents \
                 through their own pages",
            ));
        }
        Ok(self.city_root.join(doc.as_str()))
    }

    /// The receipt of one save: which document, from which version to
    /// which, and how long it now is.
    fn document_written(&mut self, doc: &Address, applied: &Applied) -> Result<(), AxError> {
        self.record(
            EventKind::DocumentWritten,
            Payload::of(&DocumentWritten {
                at: doc.clone(),
                baseline: applied.baseline(),
                version: applied.version(),
                bytes: u64::try_from(applied.bytes().len()).unwrap_or(u64::MAX),
            })?,
        )
    }
}
