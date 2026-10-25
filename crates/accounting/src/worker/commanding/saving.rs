// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A page's save of one document, and a person's decision on the
//! proposal cards of one (`crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22, `crates/wire/Spec.lean` §8-72,
//! 8-73).
//!
//! Both read the document's bytes inside its lock, hand them to the
//! `documents` rule that decides, keep the version that lands in the
//! content store, replace the file whole, and then record what landed:
//! a version the page lists can always be read back by its version
//! (`crates/wire/spec/Answer/DocumentVersions.lean` §8-83). The rules are the `documents` crate's and the lock is
//! `city::document`'s; this module only puts them in order and writes
//! the lines.

use std::path::PathBuf;

use documents::{Applied, Offer};
use kernel::event::record::{DocumentWritten, ProposalDecided, SliceVerdict};
use kernel::{Address, AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;
use crate::views::document::keep;

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
        let to = self.destination(&write.doc)?;
        let applied = city::revise_document(&to.path, |held, source| {
            let applied = documents::save(source, write.baseline, &write.edits)?;
            to.admits(applied.bytes())?;
            keep(&self.city_root, applied.bytes())?;
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
    /// whatever `documents::decide` refuses; the refusal of a prompt
    /// document on the city's list, as a save meets it; a file that
    /// cannot be read or replaced; and a history that will not take a
    /// line. Nothing is written or recorded when the decision is refused.
    pub(in crate::worker) fn decide_proposals(
        &mut self,
        decisions: &wire::ProposalDecisions,
    ) -> Result<(), AxError> {
        let to = self.destination(&decisions.doc)?;
        let cards = decisions
            .decisions
            .iter()
            .map(|decision| {
                self.governance
                    .proposals
                    .open_on(&decisions.doc, &decision.proposal)
                    .map(|offer| (offer.clone(), decision.verdicts.as_slice()))
            })
            .collect::<Result<Vec<(Offer, &[SliceVerdict])>, AxError>>()?;
        let named: Vec<(&Offer, &[SliceVerdict])> = cards
            .iter()
            .map(|(offer, verdicts)| (offer, *verdicts))
            .collect();
        let landed = city::revise_document(&to.path, |held, source| {
            let decided = documents::decide(source, &named)?;
            if let Some(applied) = &decided {
                to.admits(applied.bytes())?;
                keep(&self.city_root, applied.bytes())?;
                held.replace(applied.bytes())?;
            }
            Ok(decided)
        })?;
        if let Some(applied) = landed {
            self.document_written(&decisions.doc, &applied)?;
        }
        for decision in &decisions.decisions {
            self.record(
                EventKind::ProposalDecided,
                Payload::of(&ProposalDecided {
                    proposal: decision.proposal,
                    verdicts: decision.verdicts.clone(),
                })?,
            )?;
        }
        Ok(())
    }

    /// Where a document a page names lives, refused inside the reserved
    /// subtree: the files there have doors of their own that judge what
    /// they are given before it lands (`crates/wire/Spec.lean` §8-72).
    /// The prompt documents on the city's list are the exception, judged
    /// by that list (`city::PromptDocument`).
    fn destination(&self, doc: &Address) -> Result<Destination, AxError> {
        let listed = city::PromptDocument::of(doc);
        if doc.is_reserved() && listed.is_none() {
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
        Ok(Destination {
            path: self.city_root.join(doc.as_str()),
            listed,
        })
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

/// Where a page's save or a decided card lands, and the entry of the
/// city's prompt-document list that judges what may land there.
struct Destination {
    path: PathBuf,
    listed: Option<city::PromptDocument>,
}

impl Destination {
    /// Refuses bytes the listed prompt document may not hold; a document
    /// off the list takes any text the save rules accept.
    ///
    /// # Errors
    /// What `PromptDocument::admits` refuses: an unreadable identity area
    /// in `MAYOR.md`, Mayor text that is not UTF-8.
    fn admits(&self, bytes: &[u8]) -> Result<(), AxError> {
        self.listed.as_ref().map_or(Ok(()), |doc| doc.admits(bytes))
    }
}
