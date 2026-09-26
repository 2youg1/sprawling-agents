// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `run_forked` line: written when a branch is frozen, and read back
//! when a branch of that branch is rebuilt.

use kernel::event::record::RunForked;
use kernel::{Address, AxCode, AxError, EventDraft, EventKind, Payload, RunId, Seq, TimeMs};

use super::known;
use crate::replay::VerifiedLedger;

/// The `run_forked` draft for the city Ledger; the caller supplies the
/// new run id, the room it lands in, and the clock reading.
pub fn fork_draft(
    origin: kernel::Origin,
    new_run: RunId,
    addr: Address,
    t: TimeMs,
    who: String,
) -> Result<EventDraft, AxError> {
    let data = Payload::of(&RunForked {
        from: origin.run,
        at_seq: origin.at_seq,
    })?;
    Ok(EventDraft {
        run: new_run,
        t,
        who,
        // The room is on the line because a fold that rebuilds what each
        // room's session still owes reads it here: the new run's id says
        // which run continues which, and the address says whose session
        // has been served.
        addr: Some(addr),
        kind: EventKind::RunForked,
        data,
        ig: false,
    })
}

/// Where `owner` was branched from, read off its `run_forked` line, when
/// it was. The cut must lie before that line: a lineage that pointed at
/// or past itself is a damaged history, refused rather than followed,
/// which is also what makes the rebuild's recursion end.
pub(super) fn forked_from(
    mother: &VerifiedLedger,
    owner: RunId,
    index: usize,
) -> Result<Option<Seq>, AxError> {
    let Some(line) = mother
        .lines()
        .iter()
        .take(index)
        .filter_map(known)
        .find(|record| record.run() == owner && record.kind() == EventKind::RunForked)
    else {
        return Ok(None);
    };
    let origin = line.data().read::<RunForked>()?.at_seq;
    if origin >= line.seq() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "fork",
            format!(
                "{owner} was branched at seq {}, which is not before its own run_forked at seq {}",
                origin.value(),
                line.seq().value()
            ),
        )
        .with_recovery("branch from a run whose lineage this history holds in order"));
    }
    Ok(Some(origin))
}
