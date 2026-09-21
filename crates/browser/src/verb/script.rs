// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The scripts a verb sends, and the one shape every reply comes back
//! in.
//!
//! Every script this crate sends returns one JSON string, so there is
//! one reply shape to read rather than the driver's whole remote-value
//! vocabulary. Keeping the scripts beside that reader is what makes the
//! promise checkable in one place.

use kernel::{AxCode, AxError, Payload};
use serde_json::{Value, json};

use super::read::missing;
use crate::snapshot::PageSnapshot;

pub(super) fn read_references(args: &Payload) -> Result<Vec<String>, AxError> {
    let raw = args
        .as_map()
        .get("refs")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("refs"))?;
    let mut references = Vec::new();
    for entry in raw {
        let text = entry.as_str().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a browser action",
                "a reference that is not a string",
            )
            .with_recovery("references are minted by the snapshot and look like `e1`")
        })?;
        references.push(text.to_owned());
    }
    Ok(references)
}

/// Builds the expression that measures the named nodes.
///
/// Every reference goes through the snapshot first, so a reference the
/// model invented is refused here rather than answered with a box that
/// belongs to something else.
pub(super) fn measure_script(
    snapshot: &PageSnapshot,
    references: &[String],
) -> Result<String, AxError> {
    let mut parts = Vec::new();
    for reference in references {
        snapshot.resolve(reference)?;
        parts.push(format!(
            "(el => {{ const r = el ? el.getBoundingClientRect() : null; return r ? {{ ref: {}, \
             x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: \
             Math.round(r.height) }} : {{ ref: {}, x: 0, y: 0, width: 0, height: 0 }}; }})({})",
            json!(reference),
            json!(reference),
            crate::act::selector_of(snapshot, reference)?,
        ));
    }
    Ok(format!("JSON.stringify([{}])", parts.join(",")))
}

/// Reads what an evaluate came back with.
///
/// Every script this module sends returns one JSON string, so there is
/// one shape to read rather than the driver's whole remote-value
/// vocabulary.
///
/// # Errors
/// Refuses a reply whose shape this version does not read, and a string
/// that is not the JSON the script promised.
pub fn read_json(result: &Value) -> Result<Value, AxError> {
    let text = result
        .get("result")
        .and_then(|inner| inner.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read a script result",
                "no string value",
            )
            .with_recovery("check the driver's protocol version")
        })?;
    serde_json::from_str(text).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "read a script result",
            err.to_string(),
        )
        .with_recovery("the script returns JSON.stringify of its answer")
    })
}

/// Whether the console said anything the development loop should stop
/// for. Error level only: a warning is the page talking, an error is the
/// page failing.
#[must_use]
pub fn complained(console: &Value) -> bool {
    console.as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry.get("level").and_then(Value::as_str) == Some("error"))
    })
}
