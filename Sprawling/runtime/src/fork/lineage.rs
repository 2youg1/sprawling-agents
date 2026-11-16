// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::RunForked;
use kernel::{
    Address, AxCode, AxError, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs,
};

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

        addr: Some(addr),
        kind: EventKind::RunForked,
        data,
        ig: false,
    })
}

pub(super) fn forked_from(owner: RunId, records: &[EventRecord]) -> Result<Option<Seq>, AxError> {
    let Some(line) = records
        .iter()
        .find(|record| record.kind() == EventKind::RunForked)
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
