// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a checkpoint line holds, and for a committed one the calls the
//! commit is the result of and the commit it is compared with, as a
//! playback bundle's reader may see them (accounting-SPEC.md 8-12 and
//! 8-17, decisions 24(f) and 29(d)).
//!
//! Attribution is `accounting::trace`'s: this module asks it and writes
//! the answer down, each call by what the reader may see of its line.
//! The answer is taken only when it is about the line in hand, because
//! the same oid can be announced again, and the trace then speaks of the
//! later announcement's span.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::event::record::CheckpointCommitted;
use kernel::{AxError, EventKind, EventRecord, GitOid, Seq};

use super::document::{Base, CallTrace, Checkpoint, Cited, Decimal, Event, Holds, Near, Related};
use super::links::Links;
use super::reader::Readership;
use crate::views::commits::commit_facts;

/// What the bundle already knows about the lines a trace may name.
pub(super) struct Known<'a> {
    /// The bundle's `events`, in ascending seq.
    pub(super) events: &'a [Event],
    /// The `tool_called` lines the reader may not see.
    pub(super) hidden: &'a BTreeSet<Seq>,
    /// The lines that announce a commit an earlier line announced.
    pub(super) repeated: &'a BTreeSet<Seq>,
    pub(super) links: &'a Links,
}

/// Fills each committed checkpoint's base, diff and trace, read from the
/// city at `city_root`: its trace first, because the base is the trace's
/// previous commit, then the diff from that base.
pub(super) fn attach(
    checkpoints: &mut [Checkpoint],
    city_root: &Path,
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
            let announced = Seq::new(checkpoint.seq.0);
            let (found, attributed) = if known.repeated.contains(&announced) {
                (Base::Absent, CallTrace::Untraced)
            } else {
                traced(city_root, *oid, announced, known)
            };
            *diff = super::diff::changes(city_root, found, *oid, files, readership);
            *base = found;
            *trace = attributed;
        }
    }
}

/// The base the commit `oid`, first announced at `announced`, is compared
/// with, and the calls it is the result of. A trace that fails is written
/// down as unread, so a history the trace cannot read past the cutoff
/// still exports.
fn traced(city_root: &Path, oid: GitOid, announced: Seq, known: &Known<'_>) -> (Base, CallTrace) {
    let first = known
        .line_at(announced)
        .map(|event| EventRecord::parse_line(event.line.as_bytes()))
        .transpose()
        .and_then(|line| match line {
            Some(line) => crate::trace::trace_first(city_root, oid, &line),
            None => Ok(None),
        });
    match first {
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
/// (accounting-SPEC.md 8-12, decision 24(f)).
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
