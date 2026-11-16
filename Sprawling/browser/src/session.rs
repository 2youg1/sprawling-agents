// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use crate::port::{BrowserPort, Frame, Reply};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContextId(String);

impl ContextId {

    pub fn parse(raw: &str) -> Result<ContextId, AxError> {
        if raw.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a browsing context",
                "empty id",
            )
            .with_recovery("take the id from `browsingContext.getTree`"));
        }
        Ok(ContextId(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionRequest {

    pub network: bool,
}

#[derive(Debug, Default)]
pub struct Session {
    next: u64,
}

impl Session {
    #[must_use]
    pub fn new() -> Session {
        Session { next: 0 }
    }

    fn mint(&mut self) -> u64 {
        self.next = self.next.saturating_add(1);
        self.next
    }

    pub fn begin(&mut self, request: SessionRequest) -> Result<Frame, AxError> {
        let mut events = Vec::new();
        if request.network {
            events.push(Value::String("network".to_owned()));
        }
        Frame::new(
            self.mint(),
            "session.new",
            json!({ "capabilities": {}, "events": Value::Array(events) }),
        )
    }

    pub fn frame(&mut self, method: &str, params: Value) -> Result<Frame, AxError> {
        Frame::new(self.mint(), method, params)
    }

    pub fn tree(&mut self) -> Result<Frame, AxError> {
        Frame::new(self.mint(), "browsingContext.getTree", json!({}))
    }

    pub fn navigate(&mut self, context: &ContextId, url: &str) -> Result<Frame, AxError> {
        if url.is_empty() {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "navigate", "empty url")
                    .with_recovery("pass an absolute url"),
            );
        }
        Frame::new(
            self.mint(),
            "browsingContext.navigate",
            json!({ "context": context.as_str(), "url": url, "wait": "complete" }),
        )
    }

    pub fn evaluate(&mut self, context: &ContextId, expression: &str) -> Result<Frame, AxError> {
        Frame::new(
            self.mint(),
            "script.evaluate",
            json!({
                "expression": expression,
                "target": { "context": context.as_str() },
                "awaitPromise": true,
            }),
        )
    }

    pub fn end(&mut self) -> Result<Frame, AxError> {
        Frame::new(self.mint(), "session.end", json!({}))
    }

    pub fn read_tree(result: &Value) -> Result<Vec<ContextId>, AxError> {
        let contexts = result
            .get("contexts")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::WireMismatch,
                    "read the context tree",
                    "no `contexts` array",
                )
                .with_recovery("check the driver's protocol version")
            })?;
        let mut out = Vec::new();
        for entry in contexts {
            let raw = entry
                .get("context")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AxError::failure(
                        AxCode::WireMismatch,
                        "read the context tree",
                        "a context without an id",
                    )
                    .with_recovery("check the driver's protocol version")
                })?;
            out.push(ContextId::parse(raw)?);
        }
        Ok(out)
    }
}

#[derive(Debug, Default)]
pub struct Recording {
    answers: BTreeMap<String, Value>,

    missed: Vec<String>,
}

impl Recording {
    #[must_use]
    pub fn new() -> Recording {
        Recording::default()
    }

    pub fn answer(&mut self, frame: &Frame, result: Value) {
        self.answers.insert(key_of(frame), result);
    }

    #[must_use]
    pub fn missed(&self) -> &[String] {
        &self.missed
    }
}

fn key_of(frame: &Frame) -> String {
    format!("{} {}", frame.method(), frame.params())
}

impl BrowserPort for Recording {
    fn send(&mut self, frame: &Frame) -> Result<Reply, AxError> {
        match self.answers.get(&key_of(frame)) {
            Some(result) => Ok(Reply::Success {
                id: frame.id(),
                result: result.clone(),
            }),
            None => {
                self.missed.push(key_of(frame));
                Err(AxError::failure(
                    AxCode::BrowserUnavailable,
                    "replay a browser command",
                    key_of(frame),
                )
                .with_recovery(
                    "record this command against a real browser before replaying it offline",
                ))
            }
        }
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

    #[test]
    fn a_conversation_numbers_its_own_frames_and_never_reuses_one() {
        let mut session = Session::new();
        let begin = session.begin(SessionRequest::default()).unwrap();
        let tree = session.tree().unwrap();
        let end = session.end().unwrap();
        assert_eq!((begin.id(), tree.id(), end.id()), (1, 2, 3));
        assert_eq!(begin.method(), "session.new");
    }

    #[test]
    fn a_session_that_wants_nothing_subscribes_to_nothing() {
        let mut session = Session::new();
        let quiet = session.begin(SessionRequest::default()).unwrap();
        assert_eq!(quiet.params().get("events"), Some(&json!([])));
        let loud = session.begin(SessionRequest { network: true }).unwrap();
        assert_eq!(loud.params().get("events"), Some(&json!(["network"])));
    }

    #[test]
    fn navigation_waits_for_the_document_rather_than_the_request() {
        let mut session = Session::new();
        let context = ContextId::parse("c1").unwrap();
        let frame = session.navigate(&context, "https://example.test/").unwrap();
        assert_eq!(
            frame.params().get("wait").and_then(Value::as_str),
            Some("complete"),
            "a page still arriving reads the same as one that finished"
        );
        assert!(session.navigate(&context, "").is_err());
    }

    #[test]
    fn the_recording_answers_what_was_asked_and_says_what_it_cannot() {
        let mut session = Session::new();
        let tree = session.tree().unwrap();
        let mut recording = Recording::new();
        recording.answer(&tree, json!({ "contexts": [{ "context": "c1" }] }));

        let mut replay = Session::new();
        let asked = replay.tree().unwrap();
        let result = recording.send(&asked).unwrap().into_result().unwrap();
        assert_eq!(
            Session::read_tree(&result).unwrap(),
            vec![ContextId::parse("c1").unwrap()]
        );

        let unknown = replay
            .navigate(&ContextId::parse("c1").unwrap(), "https://elsewhere.test/")
            .unwrap();
        let err = recording.send(&unknown).unwrap_err();
        assert_eq!(err.code(), &AxCode::BrowserUnavailable);
        assert_eq!(recording.missed().len(), 1);
    }

    #[test]
    fn the_recording_answers_the_same_frame_twice_without_consuming_it() {
        let mut session = Session::new();
        let tree = session.tree().unwrap();
        let mut recording = Recording::new();
        recording.answer(&tree, json!({ "contexts": [] }));
        let once = recording.send(&tree).unwrap();
        let twice = recording.send(&tree).unwrap();
        assert_eq!(
            once, twice,
            "a replay is re-enterable; answering is not a take"
        );
        assert_eq!(once.id(), tree.id());
    }

    #[test]
    fn a_tree_this_version_cannot_read_is_refused_rather_than_returned_empty() {
        assert!(Session::read_tree(&json!({})).is_err());
        assert!(Session::read_tree(&json!({ "contexts": [{}] })).is_err());
        assert_eq!(
            Session::read_tree(&json!({ "contexts": [] })).unwrap(),
            Vec::new()
        );
    }
}
