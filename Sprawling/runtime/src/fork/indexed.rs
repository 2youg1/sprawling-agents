// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxCode, AxError, EventKind, Seq};
use storage::{CheckedLine, LedgerIndex, StorageError};

use super::{Inherited, fold_run, forked_from, no_start};
use crate::conversation::Conversation;

pub fn inherited_indexed(
    index: &LedgerIndex,
    dir: &Path,
    at_seq: Seq,
) -> Result<Inherited, AxError> {
    let mut reader = index.reader(dir);
    let owner = match reader.line_at(at_seq) {
        Ok(line) => match storage::read_line(&line) {
            Ok(CheckedLine::Known(record)) => record.run(),
            Ok(CheckedLine::IgnoredUnknown(_)) => return Err(outside(index, at_seq)),
            Err(fault) => return Err(fault.into_ax(at_seq.value().saturating_add(1))),
        },
        Err(StorageError::SeqMissing { .. }) => return Err(outside(index, at_seq)),
        Err(other) => return Err(other.into_ax()),
    };
    let mut seqs: Vec<Seq> = index.run_seqs_before(owner, Some(at_seq.next()?)).collect();
    seqs.reverse();

    let records = seqs
        .into_iter()
        .map(|seq| {
            let line = reader.line_at(seq).map_err(StorageError::into_ax)?;
            match storage::read_line(&line) {
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

    let mut conversation = Conversation::new();
    if let Some(origin) = forked_from(owner, &records)? {
        let ancestry = inherited_indexed(index, dir, origin)?;
        conversation.push_inherited(&ancestry.messages);
        let mut folded = fold_run(conversation, records.iter().skip(start))?;
        folded.delivered.extend(ancestry.delivered);
        let mut warnings = ancestry.rule_warnings;
        warnings.extend(folded.rule_warnings);
        folded.rule_warnings = warnings;
        return Ok(folded);
    }
    fold_run(conversation, records.iter().skip(start))
}

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
