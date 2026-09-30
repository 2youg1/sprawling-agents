// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The script a scripted provider plays, and what each request it
//! receives is answered with (citysim-SPEC.md 8-10).
//!
//! **One script format for both players.** A reply is the wire JSON a
//! provider of the script's compatible format would send, which is what
//! `ScriptModel::from_wire` takes, so `parse` reads every reply through
//! it: a script the stand-in provider can play is a script the scripted
//! model can play, and a mistyped key is refused here rather than
//! reaching the city as a wire mismatch nobody meant to test.
//!
//! **A request is answered by its method, not by its path.** The chat
//! path of each compatible format belongs to gateway's `chat_path`, and
//! a copy here would be a second authority for it; the city lists models
//! with `GET` and talks with `POST`, which is all this needs to tell
//! apart. The path still reaches the record, where a check can read it.

mod exchange;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

use std::collections::VecDeque;

use kernel::{AxCode, AxError, DialectKind};
use serde_json::{Value, json};

use crate::script_model::ScriptModel;

pub use exchange::ScriptedProvider;

/// A provider's side of one conversation, written down before it
/// happens: which models its list names, and the replies it gives in
/// the order requests arrive. The compatible format the replies are
/// written in is read once, to check them, and not kept: the provider
/// hands a reply back as it was written.
pub struct WireScript {
    models: Vec<String>,
    replies: VecDeque<Value>,
}

impl WireScript {
    /// Reads `{"face": …, "models": […], "replies": […]}`.
    ///
    /// `face` is spelled as `kernel::DialectKind` serializes itself
    /// (`anthropic`, `open_ai`, `open_ai_responses`); an empty `models`
    /// is a provider that serves no model list.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` naming the key path that could not be read,
    /// including a reply the city's own translation cannot read.
    pub fn parse(text: &str) -> Result<WireScript, AxError> {
        let whole: Value =
            serde_json::from_str(text).map_err(|err| unreadable("the script", &err.to_string()))?;
        let face = whole
            .get("face")
            .cloned()
            .ok_or_else(|| unreadable("face", "missing"))
            .and_then(|face| {
                serde_json::from_value::<DialectKind>(face)
                    .map_err(|err| unreadable("face", &err.to_string()))
            })?;
        let models = whole
            .get("models")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("models", "missing, or not an array"))?
            .iter()
            .enumerate()
            .map(|(at, id)| {
                id.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| unreadable(&format!("models[{at}]"), "not a string"))
            })
            .collect::<Result<Vec<String>, AxError>>()?;
        let replies = whole
            .get("replies")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("replies", "missing, or not an array"))?
            .clone();
        for (at, reply) in replies.iter().enumerate() {
            ScriptModel::from_wire(face, vec![reply.clone()])
                .map_err(|err| unreadable(&format!("replies[{at}]"), &err.to_string()))?;
        }
        Ok(WireScript {
            models,
            replies: replies.into(),
        })
    }
}

/// A script as it is being played: the replies not given yet.
pub(crate) struct Replay {
    script: WireScript,
}

/// What a request asks for, read from its method alone.
#[derive(Clone, Copy)]
pub(crate) enum Asked {
    ModelList,
    Chat,
    Other,
}

impl Asked {
    pub(crate) fn of(method: &str) -> Asked {
        match method {
            "GET" => Asked::ModelList,
            "POST" => Asked::Chat,
            _other => Asked::Other,
        }
    }
}

/// What one request is answered with.
pub(crate) enum Answer {
    Models(Value),
    Reply(Value),
    Refused(Refusal),
}

/// Why a request got no scripted answer. Each carries a stable code in
/// the body, in the `error.type` field both compatible formats use.
#[derive(Clone, Copy)]
pub(crate) enum Refusal {
    NoModelList,
    ScriptExhausted,
    MethodUnanswered,
}

impl Refusal {
    fn status(self) -> (u16, &'static str) {
        match self {
            Refusal::NoModelList => (404, "Not Found"),
            Refusal::ScriptExhausted => (410, "Gone"),
            Refusal::MethodUnanswered => (405, "Method Not Allowed"),
        }
    }

    fn code(self) -> &'static str {
        match self {
            Refusal::NoModelList => "no_model_list",
            Refusal::ScriptExhausted => "script_exhausted",
            Refusal::MethodUnanswered => "method_unanswered",
        }
    }

    fn said(self) -> &'static str {
        match self {
            Refusal::NoModelList => "this scripted provider serves no model list",
            Refusal::ScriptExhausted => "every reply of this script has been given",
            Refusal::MethodUnanswered => "a scripted provider answers GET and POST only",
        }
    }
}

impl Answer {
    /// The status code and its reason phrase.
    pub(crate) fn status(&self) -> (u16, &'static str) {
        match self {
            Answer::Models(_) | Answer::Reply(_) => (200, "OK"),
            Answer::Refused(refusal) => refusal.status(),
        }
    }

    pub(crate) fn body(&self) -> Value {
        match self {
            Answer::Models(list) => list.clone(),
            Answer::Reply(reply) => reply.clone(),
            Answer::Refused(refusal) => json!({
                "error": { "type": refusal.code(), "message": refusal.said() },
            }),
        }
    }
}

impl Replay {
    pub(crate) fn new(script: WireScript) -> Replay {
        Replay { script }
    }

    /// The answer to the next request, taking a reply off the script
    /// when the request is a chat.
    pub(crate) fn answer(&mut self, asked: Asked) -> Answer {
        match asked {
            Asked::ModelList if self.script.models.is_empty() => {
                Answer::Refused(Refusal::NoModelList)
            }
            Asked::ModelList => Answer::Models(json!({
                "data": self
                    .script
                    .models
                    .iter()
                    .map(|id| json!({ "id": id }))
                    .collect::<Vec<Value>>(),
            })),
            Asked::Chat => self
                .script
                .replies
                .pop_front()
                .map_or(Answer::Refused(Refusal::ScriptExhausted), Answer::Reply),
            Asked::Other => Answer::Refused(Refusal::MethodUnanswered),
        }
    }
}

fn unreadable(path: &str, why: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "read a provider script",
        format!("{path}: {why}"),
    )
    .with_recovery(
        "write the script as citysim-SPEC.md 8-10 states: a `face`, a `models` array of \
         ids, and `replies`, each the wire JSON a provider of that face sends",
    )
}
