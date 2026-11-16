// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{AxCode, AxError, EventDraft, EventKind, EventRecord, Payload, TimeMs};

#[derive(Debug, Default)]
pub struct DanglingCalls {
    open: BTreeMap<[u8; 16], EventRecord>,
    dangling: Vec<EventRecord>,
}

impl DanglingCalls {

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

    pub fn into_calls(self) -> Vec<EventRecord> {
        let mut calls = self.dangling;
        calls.extend(self.open.into_values());
        calls.sort_by_key(|call| call.seq().value());
        calls
    }
}

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

        took_us: None,

        rendered: Some(crate::rendering::CURRENT),
        shown_bytes: None,
        attachments: Vec::new(),
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
