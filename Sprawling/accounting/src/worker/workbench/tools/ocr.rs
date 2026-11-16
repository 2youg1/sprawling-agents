// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `ocr`: a picture this run may read - a PNG in the city, or a
//! screenshot a connector stored - read by the model the person chose
//! for `ocr` (`crates/sprawling/Spec.lean` §8-142).
//!
//! Whether the tool exists is one `select(ModelTag::Ocr, policy)`, read
//! under the rules of the building the run stands in. A run in a
//! building the book gives no such model is not offered a tool that
//! could only fail.

use std::io::Read as _;
use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, BuildingPolicy, CostTier, Effect, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value, json};

use crate::worker::workbench::{Laying, Site};

/// What every refusal of this tool names as the action that failed.
const ACTION: &str = "ocr";

impl Laying {
    /// The OCR tool, when the endpoint book names a model this building
    /// may send a picture to.
    ///
    /// # Errors
    /// Propagates an adapter that cannot be built for the chosen
    /// endpoint, which would fail the run's own model calls as well.
    pub(super) fn ocr_tool(
        &self,
        site: &Site,
        reader: runtime::BoundReader,
    ) -> Result<Option<OcrTool>, AxError> {
        let policy = site.rules.policy();
        let chosen = match self.book.select(kernel::ModelTag::Ocr, policy) {
            Ok(chosen) => chosen,
            // Nothing chosen, an endpoint since detached, or a
            // confidential building whose endpoint is off this machine:
            // each means this building has no OCR, and the answer to
            // that is a table without the tool.
            Err(_no_ocr_here) => return Ok(None),
        };
        // The headers the face asks for are the ones a run's own model
        // is called with, from the one place that spells them.
        let headers = crate::worker::credentials::dialect_headers(chosen.endpoint.dialect)
            .into_iter()
            .map(|(name, value)| (name, value.spelled()))
            .collect();
        let recogniser = gateway::recogniser_for(
            &chosen,
            crate::held_vault::resolving(Arc::clone(&self.vault)),
            headers,
            self.monotonic,
        )?;
        OcrTool::new(reader, policy.clone(), recogniser).map(Some)
    }
}

/// The tool: what the run may read, the rules its picture leaves under,
/// and the facility that reads one.
pub(super) struct OcrTool {
    reader: runtime::BoundReader,
    /// The building's policy, handed to the model call as a run's own
    /// calls hand theirs.
    policy: BuildingPolicy,
    /// Behind a lock because the credential resolver inside it is `Send`
    /// and not `Sync`, and a picture is shown through `&mut`; two
    /// pictures of one run take their turns.
    recogniser: Mutex<gateway::Recogniser>,
    meta: ToolMeta,
}

impl OcrTool {
    /// # Errors
    /// Refuses a name or a parameter schema that does not build, which
    /// the literals below cannot produce.
    fn new(
        reader: runtime::BoundReader,
        policy: BuildingPolicy,
        recogniser: gateway::Recogniser,
    ) -> Result<OcrTool, AxError> {
        let params = json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "a PNG you may read: a path relative to the city root, or \
                                    the cas: locator a screenshot was stored under",
                },
            },
            "required": ["path"],
        });
        let Value::Object(params) = params else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                "the parameter schema is not an object",
            )
            .with_recovery("report this against accounting::worker::workbench::tools::ocr"));
        };
        Ok(OcrTool {
            reader,
            policy,
            recogniser: Mutex::new(recogniser),
            meta: ToolMeta {
                name: ToolName::parse(ACTION)?,
                disclosure: "Read the text in a PNG picture - a file you may read, or a \
                             screenshot by its cas: locator - through the model this city \
                             chose for OCR. The picture is sent to that model's endpoint."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Read,
                // One provider round trip: seconds, and it may be billed.
                cost_tier: CostTier::Heavy,
                // The endpoint's own call deadline bounds the call, and a
                // second one here would be a second authority on it.
                timeout: None,
                // What comes back is text.
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// The picture `asked` names, read through the one door every
    /// model-chosen name is judged at and recognised the one way a
    /// picture is.
    ///
    /// # Errors
    /// What the door refuses the name with; `E_STORAGE_FATAL` when the
    /// bytes it opened do not read; what `png_picture` refuses.
    fn picture(&self, asked: &str) -> Result<gateway::Picture, AxError> {
        let mut bytes = Vec::new();
        self.reader
            .open(asked, ACTION)?
            .read_to_end(&mut bytes)
            .map_err(|err| {
                AxError::failure(AxCode::StorageFatal, ACTION, format!("{asked}: {err}"))
                    .with_recovery("the User has to make the file readable")
            })?;
        let seen = runtime::png_picture(&bytes)?;
        gateway::Picture::new(seen, bytes)
    }
}

impl Tool for OcrTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let asked = call
            .args
            .as_map()
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    ACTION,
                    "missing string argument `path`",
                )
                .with_recovery("pass one string: the path or the cas: locator of a PNG")
            })?;
        let picture = self.picture(asked)?;
        let text = self
            .recogniser
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    ACTION,
                    "the OCR facility was left locked by a call that died",
                )
                .with_recovery(
                    "end this run and resume it: a facility a thread died holding is not \
                     trusted again inside this process",
                )
            })?
            .recognise(picture, &self.policy)?;
        let mut out = Map::new();
        out.insert("path".to_owned(), Value::String(asked.to_owned()));
        out.insert("text".to_owned(), Value::String(text));
        Ok(ToolOutcome {
            result: Payload::new(out)?,
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
    use std::path::Path;

    use kernel::{Address, ReadVerdict};

    use super::*;

    /// A one-pixel PNG.
    const ONE_PIXEL_PNG: [u8; 70] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60,
        0x60, 0x60, 0xF8, 0x0F, 0x00, 0x01, 0x04, 0x01, 0x00, 0x5F, 0xE5, 0xC3, 0x4B, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    /// A tool over a city in a temporary directory, where `vault` is a
    /// confidential building seen from outside, with no model behind it:
    /// every case here is refused before anything would be sent.
    fn tool(root: &Path) -> OcrTool {
        let bound: runtime::ReadBound = Arc::new(|addr: &Address| {
            if addr.as_str().starts_with("vault") {
                ReadVerdict::Confidential
            } else {
                ReadVerdict::Open
            }
        });
        let store = kernel::layout::CityLayout::new(root).cas();
        OcrTool::new(
            runtime::BoundReader::new(root, bound, &store),
            BuildingPolicy::default(),
            gateway::Recogniser::absent(),
        )
        .unwrap()
    }

    fn call(path: &str) -> ToolCall {
        ToolCall {
            id: "call-1".to_owned(),
            name: ToolName::parse(ACTION).unwrap(),
            args: Payload::new(json!({ "path": path }).as_object().cloned().unwrap()).unwrap(),
        }
    }

    fn put(root: &Path, at: &str, bytes: &[u8]) {
        let path = at
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// What the door closes, what is not there and what is not a PNG are
    /// each refused with their own code before any model is asked; a PNG
    /// another open building holds reaches the facility, which here has
    /// no model and says so by name.
    #[test]
    fn a_picture_is_judged_by_the_door_and_recognised_before_it_is_sent() {
        let city = tempfile::tempdir().unwrap();
        put(city.path(), "vault/room/shot.png", &ONE_PIXEL_PNG);
        put(city.path(), "hall/room/notes.png", b"not a picture");
        put(city.path(), "hall/room/shot.png", &ONE_PIXEL_PNG);
        let tool = tool(city.path());
        let stray = format!("cas:b3-{}", kernel::B3Hash::digest(b"never stored"));
        let answered: Vec<(AxCode, String)> = [
            "vault/room/shot.png",
            "hall/.sprawling/shot.png",
            stray.as_str(),
            "hall/room/absent.png",
            "hall/room/notes.png",
            "hall/room/shot.png",
        ]
        .into_iter()
        .map(|asked| {
            let err = tool.invoke(&call(asked)).unwrap_err();
            (*err.code(), err.action().to_owned())
        })
        .collect();
        assert_eq!(
            answered,
            [
                (AxCode::GateDenied, "ocr"),
                (AxCode::GateDenied, "ocr"),
                (AxCode::GateDenied, "ocr"),
                (AxCode::InvalidArgs, "ocr"),
                (AxCode::InvalidArgs, "read a picture"),
                (AxCode::ToolUnavailable, "read the text in a picture"),
            ]
            .map(|(code, action)| (code, action.to_owned()))
        );
    }
}
