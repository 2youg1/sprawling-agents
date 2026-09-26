// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Rebuilding a branch's conversation through the ledger's resident
//! index: the named line for its run, then that run's own lines, and no
//! other line read (runtime-SPEC.md 8-2).
//!
//! The ledger's only writer asks this, and it verified the ledger when
//! it opened it; verifying the whole chain again to read one run's lines
//! would cost a read of the whole history on every branch.

use std::path::Path;

use kernel::{AxCode, AxError, EventKind, Seq};
use memory::{CheckedLine, LedgerIndex, MemoryError};

use super::{Inherited, fold_run, no_start};

/// What a new session inherits, rebuilt from the lines `index` places.
///
/// # Errors
/// Refuses an `at_seq` the index does not hold, or whose line is not a
/// record this build reads, and a run with no `run_started` before the
/// cut - the refusals [`super::inherited`] gives. Propagates a segment
/// that cannot be read and a line of the mother's that does not parse.
pub fn inherited_indexed(
    index: &LedgerIndex,
    dir: &Path,
    at_seq: Seq,
) -> Result<Inherited, AxError> {
    let mut reader = index.reader(dir);
    let owner = match reader.line_at(at_seq) {
        Ok(line) => match memory::read_line(&line) {
            Ok(CheckedLine::Known(record)) => record.run(),
            Ok(CheckedLine::IgnoredUnknown(_)) => return Err(outside(index, at_seq)),
            Err(fault) => return Err(fault.into_ax(at_seq.value().saturating_add(1))),
        },
        Err(MemoryError::SeqMissing { .. }) => return Err(outside(index, at_seq)),
        Err(other) => return Err(other.into_ax()),
    };
    let mut seqs: Vec<Seq> = index.run_seqs_before(owner, Some(at_seq.next()?)).collect();
    seqs.reverse();
    // A line of a newer kind is skipped and a malformed one refused, by
    // the rule `memory::read_line` shares with the verified door.
    let records = seqs
        .into_iter()
        .map(|seq| {
            let line = reader.line_at(seq).map_err(MemoryError::into_ax)?;
            match memory::read_line(&line) {
                Ok(CheckedLine::Known(record)) => Ok(Some(record)),
                Ok(CheckedLine::IgnoredUnknown(_)) => Ok(None),
                Err(fault) => Err(fault.into_ax(seq.value().saturating_add(1))),
            }
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>, AxError>>()?;
    let start = records
        .iter()
        .position(|record| record.kind() == EventKind::RunStarted)
        .ok_or_else(|| no_start(owner))?;
    fold_run(records.iter().skip(start))
}

/// The refusal for a line the index does not hold, in the words the
/// verified door uses: what the sequence ends at.
fn outside(index: &LedgerIndex, at_seq: Seq) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "fork",
        format!("at_seq {}", at_seq.value()),
    )
    .with_recovery(match index.tail_seq() {
        Some(tail) => format!("the mother sequence ends at seq {}", tail.value()),
        None => "the mother sequence is empty".to_string(),
    })
}
