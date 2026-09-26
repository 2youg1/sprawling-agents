// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rounds of one run and the calls in each, folded the first time
//! the person opens the run (sprawling-SPEC.md 8-91).

use std::collections::BTreeMap;

use channels::Turn;
use kernel::{AxError, EventRecord, RunId};
use serde_json::json;

use super::arrange::{Entry, NodeKey};
use super::follow::Row;

/// What each run the person has opened folded into.
pub(super) type Rounds = BTreeMap<RunId, Result<Vec<Turn>, AxError>>;

/// The turns of `run`, through the fold the rounds page uses.
///
/// # Errors
/// The first line of `run` that does not parse as a Ledger record.
pub(super) fn fold(run: RunId, rows: &[Row]) -> Result<Vec<Turn>, AxError> {
    let records = rows
        .iter()
        .filter(|row| row.run == run)
        .map(|row| EventRecord::parse_line(row.line.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sprawling::turns(&records))
}

/// Pushes the rounds of the run entry at `at`, each followed by its
/// calls; an unreadable run gets one entry naming the failure instead.
pub(super) fn append_below(
    entries: &mut Vec<Entry>,
    at: usize,
    folded: &Result<Vec<Turn>, AxError>,
) {
    let Some((NodeKey::Run(run), depth, seq)) = entries
        .get(at)
        .map(|entry| (entry.key.clone(), entry.depth.saturating_add(1), entry.seq))
    else {
        return;
    };
    let turns = match folded {
        Ok(turns) => turns,
        Err(failure) => {
            entries.push(Entry {
                key: NodeKey::Round(run, 0),
                depth,
                parent: Some(at),
                seq,
                label: format!("rounds unreadable: {failure}"),
                detail: serde_json::to_value(failure).unwrap_or_else(unserializable),
            });
            return;
        }
    };
    for turn in turns {
        let round_at = entries.len();
        entries.push(Entry {
            key: NodeKey::Round(run, turn.number),
            depth,
            parent: Some(at),
            seq: turn.opened,
            label: format!("round {} @{}", turn.number, turn.opened.value()),
            detail: serde_json::to_value(turn).unwrap_or_else(unserializable),
        });
        entries.extend(turn.calls.iter().map(|call| Entry {
            key: NodeKey::Call(run, call.at),
            depth: depth.saturating_add(1),
            parent: Some(round_at),
            seq: call.at,
            label: call.subject.as_ref().map_or_else(
                || call.tool.clone(),
                |subject| format!("{} {subject}", call.tool),
            ),
            detail: serde_json::to_value(call).unwrap_or_else(unserializable),
        }));
    }
}

fn unserializable(failure: serde_json::Error) -> serde_json::Value {
    json!({ "error": failure.to_string() })
}
