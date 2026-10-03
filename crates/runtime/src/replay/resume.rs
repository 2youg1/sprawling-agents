// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Crash recovery over verified records: which tool calls have no
//! outcome, and the line that closes one.
//!
//! A `tool_called` with no later `tool_result` in the same run is a
//! call whose outcome nobody knows. Detection and repair sit in one
//! file because they are one rule read twice — what counts as dangling
//! decides what the closing line must say.

use std::collections::BTreeMap;

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{AxCode, AxError, EventDraft, EventKind, EventRecord, Payload, TimeMs};

/// The crash-recovery detection half of resume, fed one verified record
/// at a time so it rides the pass that verifies the chain: a
/// `tool_called` with no later `tool_result` in the same run is a call
/// whose outcome is unknown, and so is a call its own run followed with
/// another call before any result.
///
/// What it holds is the calls still open, one per run, and the calls
/// already found dangling, never the ledger.
#[derive(Debug, Default)]
pub struct DanglingCalls {
    open: BTreeMap<[u8; 16], EventRecord>,
    dangling: Vec<EventRecord>,
}

impl DanglingCalls {
    /// Reads one record, in ledger order.
    pub fn observe(&mut self, record: &EventRecord) {
        let key = *record.run().as_bytes();
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "a call and its result open and close a pairing; no other kind opens or closes one"
        )]
        match record.kind() {
            EventKind::ToolCalled => {
                if let Some(older) = self.open.insert(key, record.clone()) {
                    self.dangling.push(older);
                }
            }
            EventKind::ToolResult => {
                self.open.remove(&key);
            }
            _ => {}
        }
    }

    /// The calls with no outcome, ascending by seq, once every record
    /// has been observed.
    pub fn into_calls(self) -> Vec<EventRecord> {
        let mut calls = self.dangling;
        calls.extend(self.open.into_values());
        calls.sort_by_key(|call| call.seq().value());
        calls
    }
}

/// The repair half: the `tool_result` draft that closes a dangling call
/// with `E_TOOL_OUTCOME_UNKNOWN`. The resume path appends it before any
/// new turn — the account never shows a call without an outcome.
pub fn outcome_unknown_draft(call: &EventRecord, t: TimeMs) -> Result<EventDraft, AxError> {
    if call.kind() != EventKind::ToolCalled {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "draft unknown outcome",
            "record is not a tool_called",
        )
        .with_recovery(
            "pass the `tool_called` record that has no `tool_result` after it; only a \
             call can be closed as an unknown outcome",
        ));
    }
    // Read through the one struct `tool_called` is written from, so a
    // call this build cannot read is refused rather than closed against
    // a name that names nothing and paired with an id no line answers.
    let called = call.data().read::<ToolCalled>()?;
    let error = AxError::failure(
        AxCode::ToolOutcomeUnknown,
        "recover tool outcome",
        format!("{name} ({id})", name = called.name.as_str(), id = called.id),
    )
    .with_recovery(
        "the call may or may not have taken effect; verify the external state before retrying",
    );
    let answer = ToolAnswer::Failed {
        error: Payload::of(&error)?,
    };
    let data = Payload::of(&ToolResult {
        tool_use_id: called.id.clone(),
        name: called.name.clone(),
        answer,
        // The call's own end was never seen: this answer is the city's
        // verdict at reopening, so no duration belongs to it.
        took_us: None,
    })?;
    Ok(EventDraft {
        run: call.run(),
        t,
        who: call.who().to_owned(),
        addr: None,
        kind: EventKind::ToolResult,
        data,
        ig: false,
    })
}
