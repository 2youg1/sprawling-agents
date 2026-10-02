// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The proposal cards a city's history holds, open or handled, and the
//! answer that lists one document's open cards (`crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22,
//! `crates/wire/Spec.lean` §8-73).
//!
//! Held inside [`super::Governance`], so the worker that judges a
//! decision and the page that draws a card read one fold: a card is a
//! thing waiting on a person, as an approval is. What a card is - its
//! identity, its sentences, what a verdict makes of it - is the
//! `documents` crate's; this module only remembers which cards are open.

use std::collections::BTreeMap;
use std::path::Path;

use documents::Offer;
use kernel::event::record::{ProposalDecided, ProposalOffered, ProposalWithdrawn};
use kernel::{Address, AxCode, AxError, B3Hash, Payload, RunId};

/// Every card the history offered: the open ones whole, the handled
/// ones by identity alone.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Proposals {
    /// How many cards were ever offered: the place the next one takes,
    /// so a document's cards are answered in the order they came.
    offered: u64,
    open: BTreeMap<B3Hash, Open>,
    handled: BTreeMap<B3Hash, Handled>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Open {
    place: u64,
    offer: Offer,
}

/// What became of a card that is no longer open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Handled {
    Decided,
    Withdrawn,
}

impl Proposals {
    /// Folds a `proposal_offered` line `run` wrote. A card already known,
    /// open or handled, is the same card offered again and changes
    /// nothing (documents D13).
    ///
    /// # Errors
    /// A payload this build cannot read, and an offer `documents`
    /// refuses: either is a line that cannot be shown as a card.
    pub(crate) fn offered(&mut self, run: RunId, payload: &Payload) -> Result<(), AxError> {
        let offer = Offer::of(run, &payload.read::<ProposalOffered>()?)?;
        let id = offer.id();
        if self.handled.contains_key(&id) || self.open.contains_key(&id) {
            return Ok(());
        }
        let place = self.offered;
        self.offered = self.offered.saturating_add(1);
        self.open.insert(id, Open { place, offer });
        Ok(())
    }

    /// Folds a `proposal_decided` line.
    ///
    /// # Errors
    /// A payload this build cannot read.
    pub(crate) fn decided(&mut self, payload: &Payload) -> Result<(), AxError> {
        let decided = payload.read::<ProposalDecided>()?;
        self.close(decided.proposal, Handled::Decided);
        Ok(())
    }

    /// Folds a `proposal_withdrawn` line.
    ///
    /// # Errors
    /// A payload this build cannot read.
    pub(crate) fn withdrawn(&mut self, payload: &Payload) -> Result<(), AxError> {
        let withdrawn = payload.read::<ProposalWithdrawn>()?;
        self.close(withdrawn.proposal, Handled::Withdrawn);
        Ok(())
    }

    fn close(&mut self, id: B3Hash, how: Handled) {
        self.open.remove(&id);
        self.handled.insert(id, how);
    }

    /// The open card `id` names, if it is one of `doc`'s (documents D19).
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming the card when it is not open on `doc`:
    /// decided, withdrawn, on another document, or never offered.
    pub(crate) fn open_on(&self, doc: &Address, id: &B3Hash) -> Result<&Offer, AxError> {
        let why = match self.open.get(id) {
            Some(open) if open.offer.doc() == doc => return Ok(&open.offer),
            Some(open) => format!("is a card on {}", open.offer.doc()),
            None => match self.handled.get(id) {
                Some(Handled::Decided) => "was decided already".to_owned(),
                Some(Handled::Withdrawn) => "was withdrawn by its run".to_owned(),
                None => "was never offered".to_owned(),
            },
        };
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "decide proposals",
            format!("proposal {id} {why}"),
        )
        .with_recovery("ask again which cards are open on this document"))
    }

    /// The cards open on `doc`, in the order they were offered.
    pub(crate) fn all_open_on(&self, doc: &Address) -> Vec<Offer> {
        let mut found: Vec<&Open> = self
            .open
            .values()
            .filter(|open| open.offer.doc() == doc)
            .collect();
        found.sort_by_key(|open| open.place);
        found.into_iter().map(|open| open.offer.clone()).collect()
    }
}

impl super::Views {
    /// Copies one document's open cards out under the lock; its version
    /// is read once the lock is let go.
    pub(in crate::views) fn proposals_ask(&self, doc: &Address) -> super::prepared::Prepared {
        super::prepared::Prepared::Proposals {
            city_root: self.city_root.clone(),
            doc: doc.clone(),
            open: self.governance.proposals.all_open_on(doc),
        }
    }
}

/// One document's open cards with the version its file holds now,
/// read after the view lock is let go: the version is a digest of the
/// whole file (`crates/sprawling/Spec.lean` §8-100).
pub(super) fn proposals_answer(city_root: &Path, doc: Address, open: Vec<Offer>) -> wire::Answer {
    // Missing and unreadable are both "no version now"; the document
    // answer is where a page learns which, and why (`crates/wire/Spec.lean` §8-69).
    let version = match std::fs::read(super::listing::resolve(city_root, Some(&doc))) {
        Ok(bytes) => Some(B3Hash::digest(&bytes)),
        Err(_no_version_now) => None,
    };
    let open = open
        .into_iter()
        .map(|offer| wire::ProposalCard {
            id: offer.id(),
            run: offer.run(),
            baseline: offer.baseline(),
            span: offer.span(),
            slices: offer.review().into_slices(),
        })
        .collect();
    wire::Answer::Proposals(Box::new(wire::ProposalsAnswer { doc, version, open }))
}
