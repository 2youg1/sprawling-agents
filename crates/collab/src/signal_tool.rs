// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The signal tool: the face `collab::inbox` shows a model, a thin
//! router onto the run's [`SignalDesk`], which owns what a send and a
//! pull do (`crate::signal_desk`).

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::signal_desk::{SignalDesk, text};

/// How long a send with `wait` stops its run at most, in milliseconds
/// of the injected clock (collab D9): shorter than the providers'
/// prompt-cache lifetime, so the call after a reply still reads the
/// cached prefix. A city setting that replaces it replaces this value.
pub const PATIENCE_MS: u64 = 240_000;

/// The tool itself: a thin router onto the desk, which is shared with
/// the worker because a registered tool is behind a `Box<dyn Tool>` and
/// nothing can reach into it afterwards.
pub struct SignalTool {
    meta: ToolMeta,
    desk: Arc<Mutex<SignalDesk>>,
}

impl SignalTool {
    /// # Errors
    /// Propagates a malformed tool name or parameter schema, neither of
    /// which can happen with the literals below — the fallibility is the
    /// constructors' contract, not a runtime condition.
    pub fn new(desk: Arc<Mutex<SignalDesk>>) -> Result<SignalTool, AxError> {
        let room = desk
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "reach the signal desk",
                    "the desk was left locked by a thread that died",
                )
                .with_recovery("restart this city")
            })?
            .room
            .clone();
        let mut properties = Map::new();
        for (field, description) in [
            (
                "action",
                "`send` to speak to another resident, `pull` to take what is waiting for you",
            ),
            ("to", "send only: the address you are speaking to"),
            (
                "kind",
                "send only: mention, thread, broadcast or steer; defaults to mention",
            ),
            ("text", "send only: what you want them to read"),
        ] {
            let mut spec = Map::new();
            spec.insert("type".to_owned(), Value::String("string".to_owned()));
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(field.to_owned(), Value::Object(spec));
        }
        let mut wait = Map::new();
        wait.insert("type".to_owned(), Value::String("boolean".to_owned()));
        wait.insert(
            "description".to_owned(),
            Value::String(format!(
                "send only: true when your next step needs their answer; you stop without a model \
                 call until they reply or {} s pass. Leave it out to go on at once",
                PATIENCE_MS / 1000
            )),
        );
        properties.insert("wait".to_owned(), Value::Object(wait));
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("action".to_owned())]),
        );
        Ok(SignalTool {
            meta: ToolMeta {
                name: ToolName::parse("signal")?,
                disclosure:
                    "Speak to another resident, or to every one of them, or take the signals waiting \
                     for you; call it when `status` says signals are pending."
                        .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Write { domain: room },
                cost_tier: CostTier::Free,
                timeout: None,
                render: RenderIntent::Signal,
                temporal: Temporal::Timeless,
            },
            desk,
        })
    }
}

impl Tool for SignalTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "send a signal",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let args = call.args.as_map();
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "reach the signal desk",
                "the desk was left locked by a thread that died",
            )
            .with_recovery("restart this city")
        })?;
        let result = match text(args, "action", "read a signal action")? {
            "send" => desk.send(args)?,
            "pull" => desk.pull()?,
            other => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a signal action",
                    other.to_owned(),
                )
                .with_recovery("use `send` or `pull`"));
            }
        };
        Ok(ToolOutcome {
            result,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::inbox::{Inbox, Mailslot};
    use crate::signal_desk::{Post, RoomMail};
    use kernel::{Address, RunId, TimeMs};

    /// kernel D37, collab D17: a page draws a send as who it went to and
    /// what it said, which it can only do when the line says it is one.
    #[test]
    fn the_signal_tool_declares_the_signal_render_intent() {
        let desk = SignalDesk::new(
            RunId::CITY,
            Address::parse("lab/potter").unwrap(),
            "potter@lab.1".to_owned(),
            Address::parse("lab").unwrap(),
            TimeMs::new(1_700_000_000_000),
            RoomMail {
                inbox: Inbox::new(64, 4),
                slot: Mailslot::default(),
                post: Post::new(|_| Ok(())),
            },
        );
        let tool = SignalTool::new(Arc::new(Mutex::new(desk))).unwrap();
        assert_eq!(tool.meta().render, RenderIntent::Signal);
    }
}
