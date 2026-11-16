// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub const PROPOSAL_ID_TAG: &[u8] = b"sprawling proposal v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SliceKind {
    Same,

    Delete,

    Insert,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    slices: Vec<Slice>,
}

impl Review {
    pub fn of(before: &str, after: &str) -> Review {
        Review {
            slices: slices::align(slices::pieces(before), slices::pieces(after)),
        }
    }

    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }

    pub fn into_slices(self) -> Vec<Slice> {
        self.slices
    }

    pub fn merged(&self, verdicts: &[SliceVerdict]) -> Result<String, AxError> {
        let mut named: BTreeMap<u32, &Verdict> = BTreeMap::new();
        for verdict in verdicts {
            if named.insert(verdict.slice, &verdict.verdict).is_some() {
                return Err(refused(verdict.slice, "is named twice"));
            }
        }
        let mut out = String::new();
        for (place, slice) in (0_u32..).zip(&self.slices) {
            match (slice.kind, named.remove(&place)) {
                (SliceKind::Same, Some(Verdict::Accept | Verdict::Amend { .. })) => {
                    return Err(refused(place, "did not change"));
                }
                (SliceKind::Delete, Some(Verdict::Amend { .. })) => {
                    return Err(refused(
                        place,
                        "is a deletion, and only an insertion is amended",
                    ));
                }
                (SliceKind::Same | SliceKind::Delete, None)
                | (SliceKind::Insert, Some(Verdict::Accept)) => {
                    slice.bytes_onto(&slice.text, &mut out)
                }
                (SliceKind::Insert, Some(Verdict::Amend { text })) => {
                    slice.bytes_onto(text, &mut out)
                }
                (SliceKind::Delete, Some(Verdict::Accept)) | (SliceKind::Insert, None) => {}
            }
        }
        match named.into_keys().next() {
            Some(place) => Err(refused(place, "is not on this card")),
            None => Ok(out),
        }
    }
}

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

    pub fn review(&self) -> Review {
        Review::of(&self.before, &self.after)
    }
}

pub fn decide(
    source: &[u8],
    cards: &[(&Offer, &[SliceVerdict])],
) -> Result<Option<Applied>, AxError> {
    let mut seen = BTreeSet::new();
    let mut edits = Vec::new();
    for (offer, verdicts) in cards {
        if !seen.insert(offer.id()) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "decide proposals",
                format!("proposal {} is named twice", offer.id()),
            )
            .with_recovery("name each card once, with all its verdicts"));
        }
        let text = offer.review().merged(verdicts)?;
        if text != offer.before {
            edits.push((*offer, text));
        }
    }
    let Some((first, _)) = edits.first() else {
        return Ok(None);
    };
    let version = B3Hash::digest(source);
    if let Some((stale, _)) = edits.iter().find(|(offer, _)| offer.baseline != version) {
        return Err(AxError::failure(
            AxCode::VersionConflict,
            "decide proposals",
            format!(
                "proposal {} was made on version {}, and the document is now {version}",
                stale.id(),
                stale.baseline
            ),
        )
        .with_recovery(
            "reject the card, or ask the run to offer it again on what the document says now",
        ));
    }
    let Reading::Text(encoding) = Reading::of(source) else {
        return Err(unmatched(first));
    };
    for (offer, _) in &edits {
        let stated = offer
            .span
            .within(source.len())
            .and_then(|(start, end)| source.get(start..end))
            .map(|bytes| encoding.decode(bytes));
        if !matches!(stated, Some(Ok(ref text)) if *text == offer.before) {
            return Err(unmatched(offer));
        }
    }
    let mut edits: Vec<TextEdit> = edits
        .into_iter()
        .map(|(offer, text)| TextEdit {
            span: offer.span,
            text,
        })
        .collect();
    edits.sort_by_key(|edit| edit.span);
    saved_on(source, version, &edits).map(Some)
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
    clippy::string_slice,
    reason = "test code"
)]
mod tests;
