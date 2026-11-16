// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use documents::{Applied, Offer};
use kernel::event::record::{DocumentWritten, ProposalDecided, SliceVerdict};
use kernel::{Address, AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;
use crate::views::document::keep;

impl RunWorker {

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

struct Destination {
    path: PathBuf,
    listed: Option<city::PromptDocument>,
}

impl Destination {

    fn admits(&self, bytes: &[u8]) -> Result<(), AxError> {
        self.listed.as_ref().map_or(Ok(()), |doc| doc.admits(bytes))
    }
}
