// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::ledger::chain_hash;
use kernel::{AxError, B3Hash, EventRecord, Seq};
use storage::{CheckedLine, LedgerIndex, LineCheck, Located};

use super::Cutoff;

pub(super) enum Walked {
    Known(EventRecord),

    Unknown(Seq),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tip {
    pub(super) seq: Seq,
    pub(super) chain_hash: B3Hash,
}

pub(super) struct Reached {
    pub(super) tip: Option<Tip>,

    pub(super) index: LedgerIndex,
}

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
