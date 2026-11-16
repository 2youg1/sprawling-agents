// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use documents::Offer;
use kernel::event::record::{ProposalDecided, ProposalOffered, ProposalWithdrawn};
use kernel::{Address, AxCode, AxError, B3Hash, EventKind, EventRecord, Payload, RunId, TimeMs};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Proposals {
    offered: u64,
    open: BTreeMap<B3Hash, Open>,
    handled: BTreeMap<B3Hash, Handled>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Open {
    place: u64,
    offer: Offer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Handled {
    Decided,
    Withdrawn,
}

impl Proposals {
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

    pub(crate) fn decided(&mut self, payload: &Payload) -> Result<(), AxError> {
        let decided = payload.read::<ProposalDecided>()?;
        self.close(decided.proposal, Handled::Decided);
        Ok(())
    }

    pub(crate) fn withdrawn(&mut self, payload: &Payload) -> Result<(), AxError> {
        let withdrawn = payload.read::<ProposalWithdrawn>()?;
        self.close(withdrawn.proposal, Handled::Withdrawn);
        Ok(())
    }

    fn close(&mut self, id: B3Hash, how: Handled) {
        self.open.remove(&id);
        self.handled.insert(id, how);
    }

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

    pub(crate) fn all_open_on(&self, doc: &Address) -> Vec<Offer> {
        let mut found: Vec<&Open> = self
            .open
            .values()
            .filter(|open| open.offer.doc() == doc)
            .collect();
        found.sort_by_key(|open| open.place);
        found.into_iter().map(|open| open.offer.clone()).collect()
    }

    fn newest_first(&self) -> Vec<&Offer> {
        let mut found: Vec<&Open> = self.open.values().collect();
        found.sort_by_key(|open| std::cmp::Reverse(open.place));
        found.into_iter().map(|open| &open.offer).collect()
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct OfferTimes(BTreeMap<B3Hash, TimeMs>);

impl OfferTimes {
    pub(super) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        if record.kind() != EventKind::ProposalOffered {
            return Ok(());
        }
        let offer = Offer::of(record.run(), &record.data().read::<ProposalOffered>()?)?;
        self.0.entry(offer.id()).or_insert(record.t());
        Ok(())
    }
}

impl super::Views {
    pub(in crate::views) fn proposals_ask(&self, doc: &Address) -> super::prepared::Prepared {
        super::prepared::Prepared::Proposals {
            ledger: self.ledger_ask(),
            doc: doc.clone(),
            open: self.governance.proposals.all_open_on(doc),
        }
    }
}

impl super::Views {
    pub(in crate::views) fn open_proposals_answer(&self) -> wire::OpenProposalsAnswer {
        let open = self
            .governance
            .proposals
            .newest_first()
            .into_iter()
            .map(|offer| wire::OfferedCard {
                doc: offer.doc().clone(),
                id: offer.id(),
                at: self.offer_times.0.get(&offer.id()).copied(),
            })
            .collect();
        wire::OpenProposalsAnswer { open }
    }
}

pub(super) fn proposals_answer(
    ledger: &super::prepared::LedgerAsk,
    doc: Address,
    open: Vec<Offer>,
) -> wire::Answer {
    let version = match std::fs::read(super::listing::resolve(&ledger.city_root, Some(&doc))) {
        Ok(bytes) => Some(B3Hash::digest(&bytes)),
        Err(_no_version_now) => None,
    };
    let offered = offered_lines(ledger, &open);
    let open = open
        .into_iter()
        .map(|offer| wire::ProposalCard {
            id: offer.id(),
            run: offer.run(),
            baseline: offer.baseline(),
            span: offer.span(),
            slices: offer.review().into_slices(),
            offered: offered.get(&offer.id()).copied(),
        })
        .collect();
    wire::Answer::Proposals(Box::new(wire::ProposalsAnswer { doc, version, open }))
}

fn offered_lines(
    ledger: &super::prepared::LedgerAsk,
    open: &[Offer],
) -> BTreeMap<B3Hash, kernel::Seq> {
    let runs: std::collections::BTreeSet<RunId> = open.iter().map(Offer::run).collect();
    runs.into_iter()
        .flat_map(|run| ledger.records_of(run))
        .filter(|record| record.kind() == EventKind::ProposalOffered)
        .filter_map(|record| {
            let offered = record.data().read::<ProposalOffered>().ok()?;
            let offer = Offer::of(record.run(), &offered).ok()?;
            Some((offer.id(), record.seq()))
        })
        .collect()
}
