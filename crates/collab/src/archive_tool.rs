// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Writing something down so the next run does not have to be told
//! again.
//!
//! Four kinds, closed: a preference, a decision, a correction, a fact. A
//! fifth would need a reason, and "it does not fit the other four" is
//! precisely the reason that lets a taxonomy rot — so the refusal names
//! the four and asks which one this is.
//!
//! Recall is reading, not remembering. The index is computed from what
//! is on the shelf and handed to this desk; nothing here keeps a second
//! copy, because the file is the one that is true.

use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall,
    ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

/// The four kinds a building remembers. Spelled here as the strings a
/// model writes, because this is the only place they are parsed.
pub const ARCHIVE_KINDS: [&str; 4] = ["preference", "decision", "correction", "fact"];

/// What the run wants remembered. Exhaustive, like the other desks'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveEffect {
    Recorded { kind: String, text: String },
}

/// One thing already on the shelf, as the worker read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub kind: String,
    pub text: String,
}

/// Where a record is written down at the moment a model makes it: its
/// `asset_archived` line first, then the file on the shelf (kernel
/// `spec/Event/Record.lean` D24).
///
/// The desk only decides what is worth recording; the ledger and the
/// disk belong to the assembly, which hands the desk this filer. It
/// answers the entry as it was filed, which is what the desk's own
/// recall reads from then on.
pub struct Filer(Box<File>);

type File = dyn FnMut(&ArchiveEffect) -> Result<Held, AxError> + Send;

impl Filer {
    /// `file` writes the line and the entry, or refuses; its refusal
    /// reaches the model unchanged and nothing is added to the shelf.
    #[must_use]
    pub fn new(
        file: impl FnMut(&ArchiveEffect) -> Result<Held, AxError> + Send + 'static,
    ) -> Filer {
        Filer(Box::new(file))
    }
}

impl std::fmt::Debug for Filer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Filer")
    }
}

/// The run's side of the building's memory.
#[derive(Debug)]
pub struct ArchiveDesk {
    room: Address,
    held: Vec<Held>,
    filer: Filer,
}

impl ArchiveDesk {
    /// `held` is the shelf as it stood at dispatch; every entry `filer`
    /// files during the run joins it.
    #[must_use]
    pub fn new(room: Address, held: Vec<Held>, filer: Filer) -> ArchiveDesk {
        ArchiveDesk { room, held, filer }
    }

    fn record(&mut self, kind: &str, text: &str) -> Result<Payload, AxError> {
        if !ARCHIVE_KINDS.contains(&kind) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "archive something",
                kind.to_owned(),
            )
            .with_recovery(format!(
                "which of these is it: {}? a fifth kind needs a reason, and \"none of the \
                 above\" is the reason that rots a taxonomy",
                ARCHIVE_KINDS.join(", ")
            )));
        }
        if text.trim().is_empty() {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "archive something", "empty text")
                    .with_recovery("say the thing itself, in one sentence a stranger could act on"),
            );
        }
        let filed = (self.filer.0)(&ArchiveEffect::Recorded {
            kind: kind.to_owned(),
            text: text.to_owned(),
        })?;
        self.held.push(filed);
        let mut result = Map::new();
        result.insert("kind".to_owned(), Value::String(kind.to_owned()));
        result.insert("recorded".to_owned(), Value::Bool(true));
        Payload::new(result)
    }

    fn recall(&self, query: &str) -> Result<Payload, AxError> {
        let needle = query.to_lowercase();
        let mut rows = Vec::new();
        for entry in &self.held {
            if !needle.is_empty() && !entry.text.to_lowercase().contains(&needle) {
                continue;
            }
            let mut row = Map::new();
            row.insert("kind".to_owned(), Value::String(entry.kind.clone()));
            row.insert("text".to_owned(), Value::String(entry.text.clone()));
            rows.push(Value::Object(row));
        }
        let mut result = Map::new();
        // How many were looked at, not only how many came back: a recall
        // that found nothing in a full archive and one that found
        // nothing in an empty one are different situations.
        result.insert(
            "searched".to_owned(),
            Value::Number(u64::try_from(self.held.len()).unwrap_or(u64::MAX).into()),
        );
        result.insert("found".to_owned(), Value::Array(rows));
        Payload::new(result)
    }
}

/// The tool: a thin router onto the desk.
pub struct ArchiveTool {
    meta: ToolMeta,
    desk: Arc<Mutex<ArchiveDesk>>,
}

impl ArchiveTool {
    /// # Errors
    /// Propagates a malformed tool name or parameter schema, neither of
    /// which the literals below can provoke.
    pub fn new(desk: Arc<Mutex<ArchiveDesk>>) -> Result<ArchiveTool, AxError> {
        let room = desk
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "reach the archive desk",
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
                "`record` to write something down, `recall` to read",
            ),
            (
                "kind",
                "record only: preference, decision, correction or fact",
            ),
            ("text", "record only: the thing itself, in one sentence"),
            ("query", "recall only: words to look for; omit to read all"),
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
        Ok(ArchiveTool {
            meta: ToolMeta {
                name: ToolName::parse("archive")?,
                disclosure: "Write down what this building should not have to be told twice, or \
                             read what it already knows."
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

impl Tool for ArchiveTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "archive something",
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
                "reach the archive desk",
                "the desk was left locked by a thread that died",
            )
            .with_recovery("restart this city")
        })?;
        let action = args.get("action").and_then(Value::as_str).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read an archive action",
                "missing string argument `action`",
            )
            .with_recovery("use `record` or `recall`")
        })?;
        let result = match action {
            "record" => {
                let kind = args.get("kind").and_then(Value::as_str).unwrap_or_default();
                let text = args.get("text").and_then(Value::as_str).unwrap_or_default();
                desk.record(kind, text)?
            }
            "recall" => desk.recall(
                args.get("query")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )?,
            other => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read an archive action",
                    other.to_owned(),
                )
                .with_recovery("use `record` or `recall`"));
            }
        };
        Ok(ToolOutcome {
            result,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A filer that keeps what it was handed, standing in for the
    /// ledger and the shelf.
    fn kept() -> (Filer, Arc<Mutex<Vec<ArchiveEffect>>>) {
        let filed = Arc::new(Mutex::new(Vec::new()));
        let keeping = Arc::clone(&filed);
        let filer = Filer::new(move |effect: &ArchiveEffect| {
            keeping.lock().unwrap().push(effect.clone());
            let ArchiveEffect::Recorded { kind, text } = effect;
            Ok(Held {
                kind: kind.clone(),
                text: text.trim().to_owned(),
            })
        });
        (filer, filed)
    }

    fn desk(held: Vec<Held>) -> Arc<Mutex<ArchiveDesk>> {
        Arc::new(Mutex::new(ArchiveDesk::new(
            Address::parse("lab/room1").unwrap(),
            held,
            kept().0,
        )))
    }

    fn call(args: Value) -> ToolCall {
        ToolCall {
            id: "tu_1".to_owned(),
            name: ToolName::parse("archive").unwrap(),
            args: Payload::new(args.as_object().unwrap().clone()).unwrap(),
        }
    }

    #[test]
    fn recording_files_once_at_the_call() {
        let (filer, filed) = kept();
        let tool = ArchiveTool::new(Arc::new(Mutex::new(ArchiveDesk::new(
            Address::parse("lab/room1").unwrap(),
            Vec::new(),
            filer,
        ))))
        .unwrap();
        tool.invoke(&call(serde_json::json!({
            "action": "record",
            "kind": "decision",
            "text": "the kiln is fired at 1240 degrees",
        })))
        .unwrap();
        assert_eq!(
            *filed.lock().unwrap(),
            vec![ArchiveEffect::Recorded {
                kind: "decision".to_owned(),
                text: "the kiln is fired at 1240 degrees".to_owned(),
            }]
        );
    }

    #[test]
    fn what_a_run_records_its_own_recall_reads_back() {
        let tool = ArchiveTool::new(desk(Vec::new())).unwrap();
        tool.invoke(&call(serde_json::json!({
            "action": "record",
            "kind": "fact",
            "text": "probe marker 7171",
        })))
        .unwrap();
        let outcome = tool
            .invoke(&call(
                serde_json::json!({ "action": "recall", "query": "7171" }),
            ))
            .unwrap();
        assert_eq!(
            outcome.result.as_map().clone(),
            serde_json::json!({
                "searched": 1,
                "found": [{ "kind": "fact", "text": "probe marker 7171" }],
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn a_fifth_kind_is_refused_with_the_four_it_could_be() {
        let shared = desk(Vec::new());
        let tool = ArchiveTool::new(Arc::clone(&shared)).unwrap();
        let refusal = tool
            .invoke(&call(serde_json::json!({
                "action": "record", "kind": "note", "text": "something",
            })))
            .unwrap_err();
        assert_eq!(refusal.code(), &AxCode::InvalidArgs);
        for kind in ARCHIVE_KINDS {
            assert!(refusal.recovery().contains(kind), "{kind}");
        }
    }

    #[test]
    fn an_empty_note_is_refused_because_nobody_could_act_on_it() {
        let shared = desk(Vec::new());
        let tool = ArchiveTool::new(Arc::clone(&shared)).unwrap();
        assert!(
            tool.invoke(&call(serde_json::json!({
                "action": "record", "kind": "fact", "text": "   ",
            })))
            .is_err()
        );
    }

    #[test]
    fn recall_says_how_many_it_looked_at_not_only_what_it_found() {
        let shared = desk(vec![
            Held {
                kind: "fact".to_owned(),
                text: "the kiln takes six hours to cool".to_owned(),
            },
            Held {
                kind: "preference".to_owned(),
                text: "glazes are mixed by weight".to_owned(),
            },
        ]);
        let tool = ArchiveTool::new(Arc::clone(&shared)).unwrap();
        let outcome = tool
            .invoke(&call(
                serde_json::json!({ "action": "recall", "query": "KILN" }),
            ))
            .unwrap();
        let map = outcome.result.as_map();
        assert_eq!(map.get("searched").and_then(Value::as_u64), Some(2));
        assert_eq!(map.get("found").and_then(Value::as_array).unwrap().len(), 1);

        let empty = ArchiveTool::new(desk(Vec::new()))
            .unwrap()
            .invoke(&call(
                serde_json::json!({ "action": "recall", "query": "kiln" }),
            ))
            .unwrap();
        assert_eq!(
            empty
                .result
                .as_map()
                .get("searched")
                .and_then(Value::as_u64),
            Some(0),
            "nothing found in an empty archive is a different situation from nothing found in a \
             full one"
        );
    }

    #[test]
    fn an_action_this_tool_does_not_have_is_refused_by_name() {
        let tool = ArchiveTool::new(desk(Vec::new())).unwrap();
        let refusal = tool
            .invoke(&call(serde_json::json!({ "action": "forget" })))
            .unwrap_err();
        assert!(refusal.recovery().contains("recall"));
    }
}
