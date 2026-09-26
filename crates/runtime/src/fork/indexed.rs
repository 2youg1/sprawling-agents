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

/// Rebuilds the mother's conversation from the ledger, through `at_seq`
/// or through the nearest safe point before it.
///
/// **The ledger is the only source, and it is enough.** A run's window is
/// folded forward by the loop that owns it, so it does not survive the
/// process; every line it was folded from does. This walks the mother's
/// own records - the opening from `run_started`, the assistant messages
/// from `model_returned`, the tool results from `tool_result`, and what
/// the person typed mid-flight from `steer_received` - and folds them
/// through the same [`crate::conversation::Conversation`] the live loop folds through, so the
/// sequence a fork starts from is the sequence the mother sent rather
/// than a second reading of the same records.
///
/// **Redaction is already applied.** The ledger holds what the city
/// wrote after scanning for credentials, so a branch inherits the text
/// the mother actually sent, which is the redacted one. That is the
/// honest reading: the bytes before redaction exist nowhere any more.
///
/// **The turn boundary's compaction is re-applied here, from the same
/// judgment.** The mother's window carries the compacted exchange of
/// each turn (`runtime::compaction::Exchange`), and this rebuild compacts
/// at the same boundary of the same turn, so a branch starts from the
/// bytes the mother sent rather than from the fuller bytes the ledger
/// kept.
///
/// **A reminder is not rebuilt, and cannot be.** The context gauge's
/// nudge is folded into the window without a record of its own, because
/// it is the city talking to the model about this run's budget. A branch
/// starts with a gauge of its own, so it starts without that nudge.
///
/// # Errors
/// Refuses an `at_seq` the index does not hold, or whose line is not a
/// record this build reads, and a run with no `run_started` before the
/// cut, in words a person can act on. Propagates a segment that cannot be
/// read and a line of the mother's that does not parse.
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

/// The refusal for a line the index does not hold, in the words a
/// person can act on: what the sequence ends at.
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
