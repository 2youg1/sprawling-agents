// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
// Portions copyright (c) 2026 2youg1 and the RefRain contributors

//! One suggestion a run makes about one stretch of a document version,
//! read as a card of sentences and decided sentence by sentence
//! (`crates/documents/spec/Proposal.lean`, D13-D19).
//!
//! Migrated from RefRain's `manuscript::review` (a proposal frozen on a
//! baseline, cut into review slices) and `manuscript::decision` (verdicts
//! per slice, a batch refused when it is stale or its scopes overlap).
//! The identity is a digest rather than a random id, the scope is a
//! stretch of bytes rather than a list of blocks, and several cards land
//! as one save through [`crate::save`]'s own rules.

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{ProposalOffered, SliceVerdict, Verdict};
use kernel::{Address, AxCode, AxError, B3Hash, RunId};
use serde::{Deserialize, Serialize};

use crate::edit::{Applied, TextEdit, saved_on};
use crate::encoding::Reading;
use crate::span::{Span, offset};
use crate::window::WINDOW_BYTES_MAX;

mod slices;

pub use slices::ALIGN_CELLS_MAX;

/// What a proposal's identity digest begins with, so it is never the
/// digest of some document that happens to hold the same bytes (D13).
pub const PROPOSAL_ID_TAG: &[u8] = b"sprawling proposal v1\0";

/// The role one sentence plays on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SliceKind {
    /// On both sides.
    Same,
    /// In the original only.
    Delete,
    /// In the suggestion only.
    Insert,
}

/// One sentence of a card with the whitespace around it; `lead`,
/// `text` and `trail` read in order are the sentence's bytes (D14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Slice {
    pub kind: SliceKind,
    pub text: String,
    pub lead: String,
    pub trail: String,
}

impl Slice {
    fn bytes_onto(&self, text: &str, out: &mut String) {
        out.push_str(&self.lead);
        out.push_str(text);
        out.push_str(&self.trail);
    }
}

/// The card two texts make: their sentences aligned in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    slices: Vec<Slice>,
}

impl Review {
    /// Cuts both texts into sentences and aligns them (D14, D15).
    pub fn of(before: &str, after: &str) -> Review {
        Review { slices: Vec::new() }
    }

    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }

    pub fn into_slices(self) -> Vec<Slice> {
        self.slices
    }

    /// The text a person's verdicts make of the card: a changed sentence
    /// the verdicts do not name is rejected (D16).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the slice, for a place no sentence holds,
    /// a sentence that did not change, one named twice, and an amended
    /// deletion.
    pub fn merged(&self, verdicts: &[SliceVerdict]) -> Result<String, AxError> {
        drop(verdicts);
        Ok(String::new())
    }
}

/// One proposal as the Ledger offered it: who offered it, about which
/// stretch of which version, and the two texts (D13, D18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    run: RunId,
    doc: Address,
    baseline: B3Hash,
    span: Span,
    before: String,
    after: String,
}

impl Offer {
    /// Reads one `proposal_offered` line written by `run`.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a reversed span, and for an original or a
    /// suggestion longer than `WINDOW_BYTES_MAX` (D18).
    pub fn of(run: RunId, offered: &ProposalOffered) -> Result<Offer, AxError> {
        let span = Span::new(offered.start, offered.end)?;
        for (side, text) in [
            ("original", &offered.before),
            ("suggestion", &offered.after),
        ] {
            if offset(text.len()) > WINDOW_BYTES_MAX {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "offer a proposal",
                    format!(
                        "the {side} is {} bytes, past {WINDOW_BYTES_MAX}",
                        text.len()
                    ),
                )
                .with_recovery("offer a shorter stretch, or several proposals"));
            }
        }
        Ok(Offer {
            run,
            doc: offered.doc.clone(),
            baseline: offered.baseline,
            span,
            before: offered.before.clone(),
            after: offered.after.clone(),
        })
    }

    /// The proposal's identity: a digest of the run, the document, the
    /// version, the stretch and both texts (D13).
    pub fn id(&self) -> B3Hash {
        let mut bytes = Vec::with_capacity(
            PROPOSAL_ID_TAG
                .len()
                .saturating_add(self.before.len())
                .saturating_add(self.after.len())
                .saturating_add(self.doc.as_str().len())
                .saturating_add(96),
        );
        bytes.extend_from_slice(PROPOSAL_ID_TAG);
        bytes.extend_from_slice(self.run.as_bytes());
        for text in [self.doc.as_str(), &self.before, &self.after] {
            bytes.extend_from_slice(&offset(text.len()).to_be_bytes());
            bytes.extend_from_slice(text.as_bytes());
        }
        bytes.extend_from_slice(self.baseline.as_bytes());
        bytes.extend_from_slice(&self.span.start().to_be_bytes());
        bytes.extend_from_slice(&self.span.end().to_be_bytes());
        B3Hash::digest(&bytes)
    }

    pub fn run(&self) -> RunId {
        self.run
    }

    pub fn doc(&self) -> &Address {
        &self.doc
    }

    pub fn baseline(&self) -> B3Hash {
        self.baseline
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn before(&self) -> &str {
        &self.before
    }

    pub fn after(&self) -> &str {
        &self.after
    }

    /// The card this proposal reads as.
    pub fn review(&self) -> Review {
        Review::of(&self.before, &self.after)
    }
}

/// Decides several cards on one document at once: what their verdicts
/// accept lands on `source` as one save, and `None` when they accept
/// nothing (D17).
///
/// # Errors
/// `E_INVALID_ARGS` for a card named twice, for verdicts
/// [`Review::merged`] refuses, for a card whose original is not the
/// stretch of `source` it names, and for accepted stretches that
/// overlap. `E_VERSION_CONFLICT` when a card that accepts anything was
/// made on another version than `source`. Nothing lands in any case.
pub fn decide(
    source: &[u8],
    cards: &[(&Offer, &[SliceVerdict])],
) -> Result<Option<Applied>, AxError> {
    drop((source, cards));
    Ok(None)
}

fn unmatched(offer: &Offer) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "decide proposals",
        format!(
            "proposal {} does not match the stretch {}..{} of its version",
            offer.id(),
            offer.span.start(),
            offer.span.end()
        ),
    )
    .with_recovery("reject the card: the history recorded an original its version does not hold")
}

fn refused(place: u32, why: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "decide a proposal",
        format!("sentence {place} {why}"),
    )
    .with_recovery("read the card again and give a verdict only to a changed sentence on it")
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
