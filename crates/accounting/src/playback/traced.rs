// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a checkpoint line holds, and for a committed one the calls the
//! commit is the result of and the commit it is compared with, as a
//! playback bundle's reader may see them (`crates/accounting/spec/Playback.lean` §8-12, `crates/accounting/spec/Playback/Traced.lean` §8-17
//! and 8-25, decisions 24(f), 29(d) and 37(b)).
//!
//! Attribution is `accounting::trace`'s: this module folds the walk's
//! verified lines into a `trace::History`, asks it about a commit at the
//! line that first announced it, and writes the answer down, each call by
//! what the reader may see of its line. Asked there, the views know
//! nothing after that line, so neither a later announcement of the same
//! oid nor a line after the cutoff changes the answer.

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

/// What a commit's evidence is read from: the history folded up to the
/// line in hand, which lines announced a commit first, and what the
/// history said at each committed checkpoint the bundle holds.
pub(super) struct Evidence {
    /// The city whose repository and history the evidence is read from.
    city_root: PathBuf,
    history: History,
    /// Every commit a line up to here announced, and the lines that
    /// announced one again.
    announced: BTreeSet<GitOid>,
    repeated: BTreeSet<Seq>,
    /// What the history said about the commit each held checkpoint
    /// announced first, as of that checkpoint's line.
    answers: BTreeMap<Seq, Result<Option<wire::CommitAnswer>, AxError>>,
}

/// What the bundle already knows about the lines a trace may name, and
/// the index the walk built, which a trace reads its lines through.
pub(super) struct Known<'a> {
    /// The bundle's `events`, in ascending seq.
    pub(super) events: &'a [Event],
    /// The `tool_called` lines the reader may not see.
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

    /// Folds one verified line in, in seq order, and notes whether it
    /// announces a commit an earlier line announced.
    pub(super) fn absorb(&mut self, record: &EventRecord) {
        self.history.absorb(record);
        if let Some((oid, _)) = commit_facts(record)
            && !self.announced.insert(oid)
        {
            self.repeated.insert(record.seq());
        }
    }

    /// Holds what the history folded so far says about `oid`, which the
    /// checkpoint at `announced` names, unless an earlier line announced
    /// it: the trace of a commit is its first announcement's.
    pub(super) fn hold(&mut self, announced: Seq, oid: GitOid) {
        if !self.repeated.contains(&announced) {
            self.answers.insert(announced, self.history.commit(oid));
        }
    }

    /// Fills each committed checkpoint's base, diff and trace: its trace
    /// first, because the base is the trace's previous commit, then the
    /// diff from that base.
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

    /// The base the commit announced at `announced` is compared with, and
    /// the calls it is the result of. A trace that fails is written down
    /// as unread, so a history the views refuse still exports.
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

/// The same run's previous commit; else the commit's one parent; else
/// nothing. Never the city's adjacent commit, which may be another run's.
fn base_of(commit: &wire::CommitAnswer) -> Base {
    match (commit.previous, commit.parents.as_deref()) {
        (Some(previous), _) => Base::Previous(previous.oid),
        (None, Some([parent])) => Base::Parent(*parent),
        (None, Some(_) | None) => Base::Absent,
    }
}

impl Known<'_> {
    /// The entry of `events` at `seq`.
    fn line_at(&self, seq: Seq) -> Option<&Event> {
        self.events
            .binary_search_by_key(&Decimal(seq.value()), |event| event.seq)
            .ok()
            .and_then(|at| self.events.get(at))
    }
}

/// One call by what the reader may see of its line.
fn cited(at: Seq, known: &Known<'_>) -> Cited {
    if known.line_at(at).is_some() {
        Cited::At(Decimal(at.value()))
    } else if known.hidden.contains(&at) {
        Cited::Withheld
    } else {
        Cited::Elsewhere(Decimal(at.value()))
    }
}

/// What a line says about a commit or a job pin: the pin read by its own
/// type, and every commit as `commit_facts`, the one place that tells
/// which lines name a commit and which oid, identifies it
/// (`crates/accounting/spec/Playback.lean` §8-12, accounting D24 (f)).
pub(super) fn checkpoint_of(record: &EventRecord) -> Result<Option<Holds>, AxError> {
    let named = commit_facts(record).map(|(oid, _)| oid);
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
