// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One strict pass over a city's Ledger, from genesis to the cutoff
//! (accounting-SPEC.md 8-12, decision 24(c)).
//!
//! Every line passes `storage::LineCheck`, the lines a selection will
//! leave out included, so a missing line, a broken chain or a repeated
//! seq refuses the whole export. The segments are read through
//! `storage::LedgerIndex::folding`, one segment resident at a time; a
//! torn tail is not a line there, which is the storage crate's rule and
//! not repaired here. The raw bytes each line was verified as are what
//! the fold receives, so nothing is read twice.

use std::path::Path;

use kernel::ledger::chain_hash;
use kernel::{AxError, B3Hash, EventRecord, Seq};
use storage::{CheckedLine, LedgerIndex, LineCheck, Located};

use super::Cutoff;

/// One verified line, as the fold receives it.
pub(super) enum Walked {
    Known(EventRecord),
    /// An ignorable line of a newer vocabulary: only its seq is read.
    Unknown(Seq),
}

/// The last line the walk verified: the cutoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tip {
    pub(super) seq: Seq,
    pub(super) chain_hash: B3Hash,
}

/// Where one walk stopped, and the index it built on the way.
pub(super) struct Reached {
    /// The line the walk stopped at; `None` for a ledger with no line.
    pub(super) tip: Option<Tip>,
    /// Every line the walk passed, by seq, and the lines after the
    /// cutoff located without being checked: a reader of this index
    /// reads past the cutoff only if it asks for a seq after it.
    pub(super) index: LedgerIndex,
}

/// Walks the ledger in `ledger_dir` up to `cutoff`, lending each line's
/// bytes and reading to `each`, and answers where it stopped.
///
/// # Errors
/// The first line that does not verify, as `LineFault::into_ax` names
/// it; a directory that cannot be read; the first error `each` returns.
/// Nothing before the failure is usable, so the caller discards it.
pub(super) fn walk(
    ledger_dir: &Path,
    cutoff: Cutoff,
    mut each: impl FnMut(&[u8], Walked) -> Result<(), AxError>,
) -> Result<Reached, AxError> {
    let mut check = LineCheck::at_genesis();
    let mut line_no = 0u64;
    let mut tip: Option<Tip> = None;
    let mut done = false;
    let index = LedgerIndex::folding(ledger_dir, |raw| {
        if done {
            return Ok(None);
        }
        line_no = line_no.saturating_add(1);
        let checked = check.advance(raw).map_err(|fault| fault.into_ax(line_no))?;
        let (seq, walked, located) = match checked {
            CheckedLine::Known(record) => {
                let seq = record.seq();
                let run = Some(record.run());
                (seq, Walked::Known(record), Some(Located { seq, run }))
            }
            CheckedLine::IgnoredUnknown(seq) => (seq, Walked::Unknown(seq), None),
        };
        tip = Some(Tip {
            seq,
            chain_hash: chain_hash(raw),
        });
        done = match cutoff {
            Cutoff::Latest => false,
            Cutoff::At(end) => seq >= end,
        };
        each(raw, walked)?;
        Ok(located)
    })?;
    Ok(Reached { tip, index })
}
