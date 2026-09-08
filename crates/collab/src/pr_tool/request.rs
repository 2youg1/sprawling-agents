// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The request a resident offers, and the record a rebuild reads back.

use kernel::{AxCode, AxError, Payload};
use serde_json::{Map, Value};

use crate::workshop::NodeId;

/// A request the city knows about: what it is called, who wrote it, and
/// which branch carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    pub node: NodeId,
    pub implementer: String,
    pub branch: String,
    /// The commit the branch stood at when the request was opened. It is
    /// the identity of the work being judged: a verifier who checked one
    /// commit has not vouched for a later one.
    pub commit: String,
}

impl OpenRequest {
    /// The `pr_opened` record, and the shape a rebuild reads back.
    ///
    /// # Errors
    /// Propagates the payload's refusal to hold what it was given.
    pub fn payload(&self) -> Result<Payload, AxError> {
        let mut map = Map::new();
        map.insert(
            "node".to_owned(),
            Value::String(self.node.as_str().to_owned()),
        );
        map.insert(
            "implementer".to_owned(),
            Value::String(self.implementer.clone()),
        );
        map.insert("branch".to_owned(), Value::String(self.branch.clone()));
        map.insert("commit".to_owned(), Value::String(self.commit.clone()));
        Payload::new(map)
    }

    /// Reads back what [`payload`](Self::payload) wrote.
    ///
    /// # Errors
    /// Refuses a payload missing any of the four fields.
    pub fn from_payload(data: &Payload) -> Result<OpenRequest, AxError> {
        let map = data.as_map();
        let text = |key: &str| -> Result<String, AxError> {
            map.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    AxError::failure(AxCode::InvalidArgs, "read an open request", key.to_owned())
                        .with_recovery(
                            "this shape is written by the same binary that reads it; report it",
                        )
                })
        };
        Ok(OpenRequest {
            node: NodeId::parse(&text("node")?)?,
            implementer: text("implementer")?,
            branch: text("branch")?,
            commit: text("commit")?,
        })
    }
}
