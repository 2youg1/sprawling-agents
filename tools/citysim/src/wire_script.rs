// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The script a scripted provider plays, and what each request it
//! receives is answered with (`tools/citysim/spec/WireScript.lean` §8-10, §8-13).
//!
//! **One script format for both players.** A reply is the wire JSON a
//! provider of the script's compatible format would send, and `parse`
//! reads every reply through `gateway::response_from_wire`, the
//! translation `ScriptModel::from_wire` and the city both use: a script
//! the stand-in provider can play is a script the scripted model can
//! play, and a mistyped key is refused here rather than reaching the
//! city as a wire mismatch nobody meant to test.
//!
//! **A request is answered by its method, not by its path.** The chat
//! path of each compatible format belongs to gateway's `chat_path`, and
//! a copy here would be a second authority for it; the city lists models
//! with `GET` and talks with `POST`, which is all this needs to tell
//! apart. The path still reaches the record, where a check can read it.
//!
//! **A chat belongs to the run whose call ids it carries back**
//! (citysim D11). The city sends a run's whole conversation
//! every turn, and every call id is written once in the script, so the
//! latest id a request carries names the run and the reply it is past.
//! What a run is given therefore depends on how far that run has got,
//! never on how its turns were interleaved with another run's.

mod exchange;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, DialectKind, ModelReturn};
use serde_json::{Value, json};

pub use exchange::ScriptedProvider;

/// A provider's side of a city's conversations, written down before
/// they happen: which models its list names, and for each run the
/// replies it is given in order. The provider hands a reply back as it
/// was written.
pub struct WireScript {
    face: DialectKind,
    models: Vec<String>,
    runs: Vec<Vec<Value>>,
    /// Every call id the replies make, and the reply that makes it.
    ids: BTreeMap<String, Turn>,
}

/// One reply of a script: the `reply`-th of run `run`, both from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Turn {
    pub(crate) run: usize,
    pub(crate) reply: usize,
}

impl WireScript {
    /// Reads `{"face": …, "models": […], "runs": [[…], …]}`.
    ///
    /// `face` is spelled as `kernel::DialectKind` serializes itself
    /// (`anthropic`, `open_ai`, `open_ai_responses`); an empty `models`
    /// is a provider that serves no model list.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` naming the key path that could not be read:
    /// a reply the city's own translation cannot read, an empty run, a
    /// call id written twice, and a reply before a run's last that calls
    /// no tool (`tools/citysim/spec/WireScript.lean` §8-13).
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
        let runs = whole
            .get("runs")
            .and_then(Value::as_array)
            .ok_or_else(|| unreadable("runs", "missing, or not an array"))?
            .iter()
            .enumerate()
            .map(|(run, given)| {
                given
                    .as_array()
                    .filter(|replies| !replies.is_empty())
                    .cloned()
                    .ok_or_else(|| unreadable(&format!("runs[{run}]"), "not a non-empty array"))
            })
            .collect::<Result<Vec<Vec<Value>>, AxError>>()?;
        let ids = call_ids(face, &runs)?;
        Ok(WireScript {
            face,
            models,
            runs,
            ids,
        })
    }

    fn reply(&self, turn: Turn) -> Option<&Value> {
        self.runs.get(turn.run)?.get(turn.reply)
    }
}

/// Every call id the runs' replies make, each read through the city's
/// own translation, and refused when the stand-in could not place the
/// requests that carry it back.
fn call_ids(face: DialectKind, runs: &[Vec<Value>]) -> Result<BTreeMap<String, Turn>, AxError> {
    let mut ids = BTreeMap::new();
    for (run, replies) in runs.iter().enumerate() {
        for (reply, wire) in replies.iter().enumerate() {
            let at = format!("runs[{run}][{reply}]");
            let read = gateway::response_from_wire(face, wire)
                .and_then(|response| ModelReturn::from_response(response, None))
                .map_err(|err| unreadable(&at, &err.to_string()))?;
            if read.calls.is_empty() && reply.saturating_add(1) < replies.len() {
                return Err(unreadable(
                    &at,
                    "calls no tool, so the run ends here and the replies after it are never given",
                ));
            }
            for call in read.calls {
                if let Some(first) = ids.insert(call.id.clone(), Turn { run, reply }) {
                    return Err(unreadable(
                        &at,
                        &format!(
                            "the call id {} is already written at runs[{}][{}]",
                            call.id, first.run, first.reply
                        ),
                    ));
                }
            }
        }
    }
    Ok(ids)
}

/// A script as it is being played: how many of its runs are open.
pub(crate) struct Replay {
    script: WireScript,
    opened: usize,
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    NoModelList,
    ScriptExhausted,
    NoRunLeft,
    RunsCrossed,
    BodyUnreadable,
    MethodUnanswered,
}

impl Refusal {
    fn status(self) -> (u16, &'static str) {
        match self {
            Refusal::NoModelList => (404, "Not Found"),
            Refusal::ScriptExhausted | Refusal::NoRunLeft => (410, "Gone"),
            Refusal::RunsCrossed => (409, "Conflict"),
            Refusal::BodyUnreadable => (400, "Bad Request"),
            Refusal::MethodUnanswered => (405, "Method Not Allowed"),
        }
    }

    fn code(self) -> &'static str {
        match self {
            Refusal::NoModelList => "no_model_list",
            Refusal::ScriptExhausted => "script_exhausted",
            Refusal::NoRunLeft => "no_run_left",
            Refusal::RunsCrossed => "runs_crossed",
            Refusal::BodyUnreadable => "body_unreadable",
            Refusal::MethodUnanswered => "method_unanswered",
        }
    }

    fn said(self) -> &'static str {
        match self {
            Refusal::NoModelList => "this scripted provider serves no model list",
            Refusal::ScriptExhausted => "every reply of this run has been given",
            Refusal::NoRunLeft => "every run of this script has been opened",
            Refusal::RunsCrossed => "this request carries call ids of two runs of the script",
            Refusal::BodyUnreadable => "the request body is not JSON",
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

    /// Whether this is a first turn that found no run left, which is
    /// the one refusal reading the script again can change (citysim D15).
    pub(crate) fn wants_more_runs(&self) -> bool {
        matches!(self, Answer::Refused(Refusal::NoRunLeft))
    }
}

/// Which run a chat's body names, by the scripted call ids it carries.
enum Carried {
    Nothing,
    Latest(Turn),
    Crossed,
}

impl Replay {
    pub(crate) fn new(script: WireScript) -> Replay {
        Replay { script, opened: 0 }
    }

    /// The answer to one request, and the reply it was placed at; a
    /// chat that opens a run opens it here.
    pub(crate) fn answer(&mut self, asked: Asked, body: &str) -> (Option<Turn>, Answer) {
        match asked {
            Asked::ModelList if self.script.models.is_empty() => {
                (None, Answer::Refused(Refusal::NoModelList))
            }
            Asked::ModelList => (
                None,
                Answer::Models(json!({
                    "data": self
                        .script
                        .models
                        .iter()
                        .map(|id| json!({ "id": id }))
                        .collect::<Vec<Value>>(),
                })),
            ),
            Asked::Chat => self.chat(body),
            Asked::Other => (None, Answer::Refused(Refusal::MethodUnanswered)),
        }
    }

    /// Takes a script read again, when it begins with every run this
    /// replay holds: the runs already open keep the replies they had.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when the face, the model list or any run held
    /// differs in the script read again.
    pub(crate) fn grow(&mut self, script: WireScript) -> Result<(), AxError> {
        let held = &self.script;
        if script.face != held.face
            || script.models != held.models
            || !script.runs.starts_with(&held.runs)
        {
            return Err(unreadable(
                "runs",
                "the script read again does not begin with the runs already being played",
            ));
        }
        self.script = script;
        Ok(())
    }

    fn chat(&mut self, body: &str) -> (Option<Turn>, Answer) {
        let Ok(sent) = serde_json::from_str::<Value>(body) else {
            return (None, Answer::Refused(Refusal::BodyUnreadable));
        };
        match self.carried(&sent) {
            Carried::Crossed => (None, Answer::Refused(Refusal::RunsCrossed)),
            Carried::Latest(past) => self.continuing(Turn {
                run: past.run,
                reply: past.reply.saturating_add(1),
            }),
            Carried::Nothing => self.opening(),
        }
    }

    /// A first turn opens the next run not opened yet.
    fn opening(&mut self) -> (Option<Turn>, Answer) {
        let first = Turn {
            run: self.opened,
            reply: 0,
        };
        match self.script.reply(first).cloned() {
            Some(reply) => {
                self.opened = self.opened.saturating_add(1);
                (Some(first), Answer::Reply(reply))
            }
            None => (None, Answer::Refused(Refusal::NoRunLeft)),
        }
    }

    /// A later turn is given the reply after the one it carries back.
    fn continuing(&self, next: Turn) -> (Option<Turn>, Answer) {
        match self.script.reply(next).cloned() {
            Some(reply) => (Some(next), Answer::Reply(reply)),
            None => (None, Answer::Refused(Refusal::ScriptExhausted)),
        }
    }

    /// The scripted call ids among the body's string values, read as
    /// the run they belong to and the latest reply among them.
    fn carried(&self, sent: &Value) -> Carried {
        let mut found = Vec::new();
        self.collect(sent, &mut found);
        let Some(first) = found.first().copied() else {
            return Carried::Nothing;
        };
        if found.iter().any(|turn| turn.run != first.run) {
            return Carried::Crossed;
        }
        found
            .into_iter()
            .max_by_key(|turn| turn.reply)
            .map_or(Carried::Nothing, Carried::Latest)
    }

    fn collect(&self, value: &Value, found: &mut Vec<Turn>) {
        match value {
            Value::String(text) => found.extend(self.script.ids.get(text).copied()),
            Value::Array(items) => items.iter().for_each(|item| self.collect(item, found)),
            Value::Object(fields) => fields.values().for_each(|item| self.collect(item, found)),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
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
        "write the script as tools/citysim/Spec.lean sections 8-10 and 8-13 state: a `face`, \
         a `models` array of ids, and `runs`, each a non-empty array of the wire JSON a \
         provider of that face sends, every call id written once, and every reply but a \
         run's last calling a tool",
    )
}
