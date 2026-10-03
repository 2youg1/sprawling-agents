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
                render: RenderIntent::Generic,
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
