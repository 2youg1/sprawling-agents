// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kernel::event::record::CheckpointCommitted;
use kernel::layout::CityLayout;
use kernel::{AxError, EventKind, EventRecord, GitOid, Seq};
use storage::LedgerIndex;

use super::document::{Base, CallTrace, Checkpoint, Cited, Decimal, Event, Holds, Near, Related};
use super::links::Links;
use super::reader::Readership;
use crate::trace::History;
use crate::views::commits::commit_facts;

pub(super) struct Evidence {

    city_root: PathBuf,
    history: History,

    announced: BTreeSet<GitOid>,
    repeated: BTreeSet<Seq>,

    answers: BTreeMap<Seq, Result<Option<wire::CommitAnswer>, AxError>>,
}

pub(super) struct Known<'a> {

    pub(super) events: &'a [Event],

    pub(super) hidden: &'a BTreeSet<Seq>,
    pub(super) links: &'a Links,
    pub(super) index: &'a LedgerIndex,
}

impl Evidence {
    pub(super) fn new(city_root: &Path) -> Evidence {
        Evidence {
            city_root: city_root.to_path_buf(),
            history: History::new(city_root),
            announced: BTreeSet::new(),
            repeated: BTreeSet::new(),
            answers: BTreeMap::new(),
        }
    }

    pub(super) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.history.absorb(record);
        if let Some((oid, _)) = commit_facts(record)?
            && !self.announced.insert(oid)
        {
            self.repeated.insert(record.seq());
        }
        Ok(())
    }

    pub(super) fn hold(&mut self, announced: Seq, oid: GitOid) {
        if !self.repeated.contains(&announced) {
            self.answers.insert(announced, self.history.commit(oid));
        }
    }

    pub(super) fn attach(
        &self,
        checkpoints: &mut [Checkpoint],
        known: &Known<'_>,
        readership: &mut Readership,
    ) {
        for checkpoint in checkpoints {
            if let Holds::Committed {
                oid,
                files,
                base,
                diff,
                trace,
                ..
            } = &mut checkpoint.holds
            {
                let (found, attributed) = self.traced(Seq::new(checkpoint.seq.0), known);
                let between = super::diff::Between {
                    base: found,
                    head: *oid,
                };
                *diff = super::diff::changes(&self.city_root, between, files, readership);
                *base = found;
                *trace = attributed;
            }
        }
    }

    fn traced(&self, announced: Seq, known: &Known<'_>) -> (Base, CallTrace) {
        let ledger_dir = CityLayout::new(&self.city_root).ledger();
        let found = match self.answers.get(&announced) {
            Some(Ok(Some(commit))) => {
                crate::trace::trace_through(known.index, &ledger_dir, commit.clone()).map(Some)
            }
            Some(Ok(None)) | None => Ok(None),
            Some(Err(refused)) => Err(refused.clone()),
        };
        match found {
            Ok(Some(found)) => (
                base_of(&found.commit),
                CallTrace::Traced {
                    calls: found
                        .calls
                        .iter()
                        .map(|call| cited(call.at, known))
                        .collect(),
                    nearby: found
                        .nearby
                        .into_iter()
                        .map(|near| {
                            let run = known.links.related(near.run);
                            Near {
                                actor: matches!(run, Related::Run(_)).then_some(near.actor),
                                run,
                                calls: Decimal(near.calls),
                            }
                        })
                        .collect(),
                },
            ),
            Ok(None) => (Base::Absent, CallTrace::Untraced),
            Err(err) => (
                Base::Absent,
                CallTrace::Unread(err.code().as_str().to_owned()),
            ),
        }
    }
}

fn base_of(commit: &wire::CommitAnswer) -> Base {
    match (commit.previous, commit.parents.as_deref()) {
        (Some(previous), _) => Base::Previous(previous.oid),
        (None, Some([parent])) => Base::Parent(*parent),
        (None, Some(_) | None) => Base::Absent,
    }
}

impl Known<'_> {

    fn line_at(&self, seq: Seq) -> Option<&Event> {
        self.events
            .binary_search_by_key(&Decimal(seq.value()), |event| event.seq)
            .ok()
            .and_then(|at| self.events.get(at))
    }
}

fn cited(at: Seq, known: &Known<'_>) -> Cited {
    if known.line_at(at).is_some() {
        Cited::At(Decimal(at.value()))
    } else if known.hidden.contains(&at) {
        Cited::Withheld
    } else {
        Cited::Elsewhere(Decimal(at.value()))
    }
}

pub(super) fn checkpoint_of(record: &EventRecord) -> Result<Option<Holds>, AxError> {
    let named = commit_facts(record)?.map(|(oid, _)| oid);
    if record.kind() == EventKind::CheckpointCommitted {
        return Ok(
            match (record.data().read::<CheckpointCommitted>()?, named) {
                (CheckpointCommitted::JobPinned { job }, _) => Some(Holds::Pinned { job }),
                (CheckpointCommitted::Committed(commit), Some(oid)) => Some(Holds::Committed {
                    oid,
                    scope: commit.scope,
                    files: commit.files,
                    base: Base::Absent,
                    diff: Vec::new(),
                    trace: CallTrace::Untraced,
                }),
                (CheckpointCommitted::Committed(_), None) => None,
            },
        );
    }
    Ok(named
        .filter(|_| record.kind() == EventKind::PrMerged)
        .map(|oid| Holds::Merged { oid }))
}
