// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fork: a new Run whose in-window history is a byte-identical prefix of
//! the mother sequence. Forking is not
//! resurrection: the mother's frozen state never changes, and the new
//! RunId arrives from the caller — this module is pure.
//!
//! Consumes [`VerifiedLedger`] only: replay and fork share one rebuilder,
//! so fork correctness and replay correctness are the same assertion.

use kernel::event::record::RunForked;
use kernel::{AxCode, AxError, EventDraft, EventKind, Payload, RunId, Seq, TimeMs};

use crate::replay::VerifiedLedger;

/// The fork prefix: raw lines `0..=at_seq`, byte-exact. Past the tail is
/// `E_INVALID_ARGS`, never a silent clamp.
pub fn prefix(mother: &VerifiedLedger, at_seq: Seq) -> Result<Vec<Vec<u8>>, AxError> {
    let refuse = || {
        AxError::failure(
            AxCode::InvalidArgs,
            "fork",
            format!("at_seq {}", at_seq.value()),
        )
        .with_recovery(match mother.tail_seq() {
            Some(tail) => format!("the mother sequence ends at seq {}", tail.value()),
            None => "the mother sequence is empty".to_string(),
        })
    };
    let tail = mother.tail_seq().ok_or_else(refuse)?;
    if at_seq > tail {
        return Err(refuse());
    }
    let index = usize::try_from(at_seq.value()).map_err(|_| refuse())?;
    let end = index.checked_add(1).ok_or_else(refuse)?;
    mother
        .raw_lines()
        .get(..end)
        .map(<[Vec<u8>]>::to_vec)
        .ok_or_else(refuse)
}

/// The `run_forked` draft for the city Ledger; the caller supplies the
/// new run id and the clock reading.
pub fn fork_draft(
    from: RunId,
    at_seq: Seq,
    new_run: RunId,
    t: TimeMs,
    who: String,
) -> Result<EventDraft, AxError> {
    let data = Payload::of(&RunForked { from, at_seq })?;
    Ok(EventDraft {
        run: new_run,
        t,
        who,
        addr: None,
        kind: EventKind::RunForked,
        data,
        ig: false,
    })
}
