// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a playback bundle is found to be (accounting-SPEC.md 8-12,
//! decision 24(d)).
//!
//! A bundle on its own can only be found consistent: that says nothing
//! about whether it was changed, because a changed bundle can be made
//! consistent again. Against another bundle it is the same bytes or not.
//! Against its city it is recomputed with the reader the caller names
//! and compared whole, so keeping `source` while changing a table does
//! not pass.

use std::collections::BTreeSet;
use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, RunId, Seq};
use storage::CheckedLine;

use super::document::{Decimal, Document, End, Event};
use super::encode::decode;
use super::reader::Reader;
use super::select::Selection;
use super::{Cutoff, PROJECTION_RULES, Projected, Request, SCHEMA, project};

/// What a bundle is checked against.
#[derive(Debug, Clone)]
pub enum Against<'a> {
    /// Nothing: the bundle is only read and checked for consistency.
    Nothing,
    /// Another bundle's bytes.
    Bundle(&'a [u8]),
    /// The city it was exported from, read by `reader`.
    City { root: &'a Path, reader: Reader },
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub digest: B3Hash,
    pub events: usize,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Readable, canonical, and every reference inside it resolves.
    Consistent,
    /// The same bytes as the bundle or the recomputation it was checked
    /// against.
    Same,
    /// Different bytes; `section` is the first part that differs.
    Differs { section: &'static str },
    /// This build cannot recompute the bundle, and says what can.
    CannotReproduce { why: String },
}

/// Checks `bytes` against `against`.
///
/// # Errors
/// `E_INVALID_ARGS` when `bytes` (or the other bundle) is not a
/// consistent bundle of this schema; whatever recomputing it from the
/// city refuses.
pub fn check(bytes: &[u8], against: Against<'_>) -> Result<Report, AxError> {
    let (document, bundle) = decode(bytes)?;
    consistent(&document)?;
    let verdict = match against {
        Against::Nothing => Verdict::Consistent,
        Against::Bundle(other) => {
            let (theirs, _) = decode(other)?;
            consistent(&theirs)?;
            compared(&document, &theirs)
        }
        Against::City { root, reader } => recomputed(&document, root, reader)?,
    };
    Ok(Report {
        digest: bundle.digest(),
        events: bundle.events(),
        verdict,
    })
}

fn recomputed(document: &Document, root: &Path, reader: Reader) -> Result<Verdict, AxError> {
    let source = &document.source;
    if source.rules != PROJECTION_RULES {
        return Ok(Verdict::CannotReproduce {
            why: format!(
                "exported under projection rules {}, and this build projects under {PROJECTION_RULES}; \
                 check it with a build of rules {}, or export it again",
                source.rules, source.rules
            ),
        });
    }
    if source.reader != reader.name() {
        return Ok(Verdict::CannotReproduce {
            why: "exported for another reader than this check reads as; check it as the reader \
                  it was exported for, or export it again"
                .to_owned(),
        });
    }
    let request = Request {
        selection: Selection::from_chosen(&source.selection)?,
        reader,
        cutoff: Cutoff::At(Seq::new(source.cutoff.seq.0)),
    };
    Ok(match project(root, &request)? {
        Projected::Whole(again) => compared(document, &again),
        Projected::EndsAt(reached) => Verdict::CannotReproduce {
            why: format!(
                "the city's ledger ends at seq {}, before the cutoff {}",
                reached.map_or_else(|| "none".to_owned(), |seq| seq.value().to_string()),
                source.cutoff.seq.0
            ),
        },
    })
}

/// `Same`, or the first section in which two documents differ.
fn compared(ours: &Document, theirs: &Document) -> Verdict {
    let sections: [(&'static str, bool); 11] = [
        ("schema", ours.schema == theirs.schema),
        ("source", ours.source == theirs.source),
        ("events", ours.events == theirs.events),
        ("context", ours.context == theirs.context),
        ("unknown", ours.unknown == theirs.unknown),
        ("runs", ours.runs == theirs.runs),
        ("moments", ours.moments == theirs.moments),
        ("messages", ours.messages == theirs.messages),
        ("checkpoints", ours.checkpoints == theirs.checkpoints),
        ("costs", ours.costs == theirs.costs),
        ("withheld", ours.withheld == theirs.withheld),
    ];
    sections
        .into_iter()
        .find(|(_, same)| !same)
        .map_or(Verdict::Same, |(section, _)| Verdict::Differs { section })
}

/// Every reference inside a document resolves, and every line is the
/// ledger line its entry says it is.
fn consistent(document: &Document) -> Result<(), AxError> {
    if document.schema != SCHEMA {
        return Err(inconsistent(format!(
            "schema '{}', where this build reads '{SCHEMA}'",
            document.schema
        )));
    }
    let events = ascending("events", &document.events)?;
    let context = ascending("context", &document.context)?;
    if let Some(both) = events.intersection(&context).next() {
        return Err(inconsistent(format!(
            "seq {} is in events and in context",
            both.0
        )));
    }
    let runs: BTreeSet<RunId> = document
        .events
        .iter()
        .map(|event| read_entry(event).map(|record| record.run()))
        .collect::<Result<_, _>>()?;
    for event in &document.context {
        read_entry(event)?;
    }
    let ends = document
        .moments
        .iter()
        .flat_map(|moment| [moment.opened, moment.closed])
        .chain(
            document
                .messages
                .iter()
                .flat_map(|message| [message.sent, message.consumed]),
        );
    for end in ends {
        match end {
            End::At(seq) if !events.contains(&seq) => {
                return Err(inconsistent(format!(
                    "an end at seq {} names no event",
                    seq.0
                )));
            }
            End::Outside(seq) if !context.contains(&seq) => {
                return Err(inconsistent(format!(
                    "an end outside the range at seq {} names no context line",
                    seq.0
                )));
            }
            End::At(_) | End::Outside(_) | End::Withheld | End::Pending | End::Missing => {}
        }
    }
    if let Some(member) = document
        .moments
        .iter()
        .flat_map(|moment| &moment.seqs)
        .find(|seq| !events.contains(seq))
    {
        return Err(inconsistent(format!(
            "a key moment names seq {}, which is no event",
            member.0
        )));
    }
    match document.runs.iter().find(|run| !runs.contains(&run.run)) {
        Some(stray) => Err(inconsistent(format!(
            "run {} has no event in the bundle",
            stray.run
        ))),
        None => Ok(()),
    }
}

/// The seqs of `entries`, refusing a list that is not strictly ascending.
fn ascending(section: &str, entries: &[Event]) -> Result<BTreeSet<Decimal>, AxError> {
    let mut seen = BTreeSet::new();
    let mut last: Option<Decimal> = None;
    for entry in entries {
        if last.is_some_and(|last| entry.seq <= last) {
            return Err(inconsistent(format!(
                "{section} are not in strictly ascending seq at {}",
                entry.seq.0
            )));
        }
        last = Some(entry.seq);
        seen.insert(entry.seq);
    }
    Ok(seen)
}

/// The record an entry's line holds, which must carry the entry's seq
/// and moment.
fn read_entry(entry: &Event) -> Result<kernel::EventRecord, AxError> {
    let record = match storage::read_line(entry.line.as_bytes()) {
        Ok(CheckedLine::Known(record)) => record,
        Ok(CheckedLine::IgnoredUnknown(_)) => {
            return Err(inconsistent(format!(
                "seq {} holds a line of a kind this build does not read",
                entry.seq.0
            )));
        }
        Err(fault) => {
            return Err(inconsistent(format!(
                "seq {} is not a ledger line: {fault:?}",
                entry.seq.0
            )));
        }
    };
    let moment = record.moment().map(|moment| Decimal(moment.value()));
    if record.seq().value() != entry.seq.0 || moment != entry.moment {
        return Err(inconsistent(format!(
            "the entry at seq {} does not match its line",
            entry.seq.0
        )));
    }
    Ok(record)
}

fn inconsistent(subject: String) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "check a playback bundle", subject).with_recovery(
        "export the bundle again with sprawling playback export; a bundle whose parts disagree \
         was changed after it was written",
    )
}
