// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One MCP server's answer as the model reads it (runtime-SPEC.md
//! 8-27-10): text that fits passes untouched, text that does not is
//! stored whole and replaced by a window the model pages with `read`.

use kernel::{AxCode, AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use crate::offload::OffloadSite;

use super::{PackContext, package};

/// The most text one connector answer puts into the window.
pub const CONNECTOR_CAP_BYTES: u64 = 16_384;

/// Packages one connector answer for the window.
///
/// An answer without a `content` array, or whose text fits
/// [`CONNECTOR_CAP_BYTES`], comes back untouched. Otherwise the text
/// blocks, joined in order, are stored whole through [`package`] and
/// replaced by one text block holding the substitute; every other block
/// follows it in its original order, and the account of the move sits
/// in the result's `offload` field.
///
/// # Errors
/// Propagates whatever [`package`] reports about the store.
pub fn package_connector(
    outcome: ToolOutcome,
    offload: OffloadSite<'_>,
) -> Result<ToolOutcome, AxError> {
    let Some(Value::Array(blocks)) = outcome.result.as_map().get("content") else {
        return Ok(outcome);
    };
    let (texts, others): (Vec<&Value>, Vec<&Value>) =
        blocks.iter().partition(|block| text_of(block).is_some());
    let text = texts
        .iter()
        .filter_map(|block| text_of(block))
        .collect::<Vec<&str>>()
        .join(
            "
",
        );
    if u64::try_from(text.len()).is_ok_and(|len| len <= CONNECTOR_CAP_BYTES) {
        return Ok(outcome);
    }
    let packaged = package(
        text.as_bytes(),
        PackContext {
            cap_bytes: CONNECTOR_CAP_BYTES,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(offload),
            sieve: None,
            adviser: None,
        },
    )?;
    let mut window = Map::new();
    window.insert("type".to_owned(), Value::String("text".to_owned()));
    window.insert("text".to_owned(), Value::String(packaged.content));
    let content = std::iter::once(Value::Object(window))
        .chain(others.into_iter().cloned())
        .collect();
    let accounts = packaged
        .events
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode offload account", err.to_string())
                .with_recovery(
                    "report this against runtime::pipeline::connector: an offload account                      holds names and counts, and JSON refuses neither",
                )
        })?;
    let mut result = outcome.result.as_map().clone();
    result.insert("content".to_owned(), Value::Array(content));
    result.insert("offload".to_owned(), Value::Array(accounts));
    Ok(ToolOutcome {
        result: Payload::new(result)?,
        attachments: outcome.attachments,
    })
}

/// The text of a block MCP marks `"type": "text"`, and nothing else.
fn text_of(block: &Value) -> Option<&str> {
    (block.get("type").and_then(Value::as_str) == Some("text"))
        .then(|| block.get("text").and_then(Value::as_str))
        .flatten()
}

#[cfg(test)]
mod tests;
