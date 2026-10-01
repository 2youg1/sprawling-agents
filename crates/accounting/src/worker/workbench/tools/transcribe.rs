// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `transcribe`: a recording this run may read - a file in the city, or
//! a block a connector stored - turned into text by the endpoint the
//! person chose to transcribe (sprawling-SPEC.md 8-131).
//!
//! Whether the tool exists is the book's answer to the question the
//! composer's microphone asks, `select(ModelTag::Transcribe, policy)`,
//! read under the rules of the building the run stands in. A run in a
//! building the book gives no transcription endpoint is not offered a
//! tool that could only fail.

use std::sync::{Arc, Mutex};

use kernel::{
    AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall, ToolMeta,
    ToolName, ToolOutcome,
};
use runtime::tools::Named;
use serde_json::{Map, Value, json};

use crate::worker::workbench::{Laying, Site};

/// What every refusal of this tool names as the action that failed.
const ACTION: &str = "transcribe";

impl Laying {
    /// The transcription tool, when the endpoint book names one this
    /// building may send a recording to.
    ///
    /// # Errors
    /// Propagates an HTTP client that cannot be built for the chosen
    /// endpoint, which would fail the run's own model calls as well.
    pub(super) fn transcription_tool(
        &self,
        site: &Site,
        reader: runtime::BoundReader,
    ) -> Result<Option<TranscribeTool>, AxError> {
        let chosen = match self
            .book
            .select(kernel::ModelTag::Transcribe, site.rules.policy())
        {
            Ok(chosen) => chosen,
            // Nothing chosen, an endpoint since detached, or a
            // confidential building whose endpoint is off this machine:
            // each means this building has no transcription, and the
            // answer to that is a table without the tool.
            Err(_no_transcription_here) => return Ok(None),
        };
        let transcriber = gateway::transcriber_for(
            &chosen,
            crate::held_vault::resolving(Arc::clone(&self.vault)),
        )?;
        TranscribeTool::new(reader, transcriber).map(Some)
    }
}

/// The tool: what the run may read, and the facility that turns a
/// recording into text.
pub(super) struct TranscribeTool {
    reader: runtime::BoundReader,
    /// Behind a lock because the credential resolver inside it is `Send`
    /// and not `Sync`, while a tool is shared between the calls of a
    /// wave; two transcriptions of one run take their turns.
    transcriber: Mutex<gateway::Transcriber>,
    meta: ToolMeta,
}

impl TranscribeTool {
    /// # Errors
    /// Refuses a name or a parameter schema that does not build, which
    /// the literals below cannot produce.
    fn new(
        reader: runtime::BoundReader,
        transcriber: gateway::Transcriber,
    ) -> Result<TranscribeTool, AxError> {
        let params = json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "a recording you may read: a path relative to the city \
                                    root ending in .webm, .ogg, .mp3, .mp4 or .wav, or the \
                                    cas: locator a recording was stored under",
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
            .with_recovery(
                "report this against accounting::worker::workbench::tools::transcribe",
            ));
        };
        Ok(TranscribeTool {
            reader,
            transcriber: Mutex::new(transcriber),
            meta: ToolMeta {
                name: ToolName::parse(ACTION)?,
                disclosure: "Turn a recording of speech into text - a file you may read, or \
                             a recording by its cas: locator - through the transcription \
                             endpoint this city was given. The recording is sent to that \
                             endpoint."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Read,
                // One provider round trip: seconds, and it may be billed.
                cost_tier: CostTier::Heavy,
                // The facility's own deadline bounds the call, and a
                // second one here would be a second authority on it.
                timeout: None,
                // What comes back is text.
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// The recording `asked` names, read through the one door every
    /// model-chosen name is judged at.
    ///
    /// A file says what container it is by its name, a `file:` Locator
    /// included; a block has no name and says it by how it starts.
    ///
    /// # Errors
    /// What the door refuses the name with, a container this city cannot
    /// send, and whatever reading the recording refuses.
    fn recording(&self, asked: &str) -> Result<gateway::Recording, AxError> {
        let opened = self.reader.open(asked, ACTION)?;
        match opened.named().clone() {
            Named::File(addr) => {
                let kind = gateway::AudioType::of_file_name(addr.as_str())?;
                gateway::Recording::read_from(opened, kind)
            }
            Named::Block(_) => gateway::Recording::read_unlabelled(opened),
        }
    }
}

impl Tool for TranscribeTool {
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
                .with_recovery("pass one string: the path or the cas: locator of a recording")
            })?;
        let recording = self.recording(asked)?;
        let text = self
            .transcriber
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::StorageFatal,
                    ACTION,
                    "the transcription facility was left locked by a call that died",
                )
                .with_recovery(
                    "end this run and resume it: a facility a thread died holding is not \
                     trusted again inside this process",
                )
            })?
            .transcribe(&recording)?;
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

    /// A tool over a city in a temporary directory, where `vault` is a
    /// confidential building seen from outside, with no endpoint behind
    /// it: every case here is refused before anything would be sent.
    fn tool(root: &Path) -> TranscribeTool {
        let bound: runtime::ReadBound = Arc::new(|addr: &Address| {
            if addr.as_str().starts_with("vault") {
                ReadVerdict::Confidential
            } else {
                ReadVerdict::Open
            }
        });
        let store = kernel::layout::CityLayout::new(root).cas();
        TranscribeTool::new(
            runtime::BoundReader::new(root, bound, &store),
            gateway::Transcriber::absent(),
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

    fn stored(root: &Path, building: &str, bytes: &[u8]) -> String {
        let hash = storage::Cas::open(&kernel::layout::CityLayout::new(root).cas())
            .unwrap()
            .put_for(
                bytes,
                &storage::BlockOrigin {
                    run: kernel::RunId::from_bytes([7; 16]),
                    building: Address::parse(building).unwrap(),
                },
            )
            .unwrap();
        format!("cas:b3-{hash}")
    }

    /// What the door closes, what is not there and a container this city
    /// cannot send are refused before anything is sent; a recording
    /// another open building holds and a stored block reach the
    /// facility, which here has no endpoint and says so by name.
    #[test]
    fn a_recording_is_judged_by_the_door_and_its_container_before_it_is_sent() {
        let city = tempfile::tempdir().unwrap();
        put(city.path(), "vault/room/voice.wav", b"RIFF-secret");
        put(city.path(), "hall/room/voice.flac", b"fLaC-spoken");
        put(city.path(), "hall/room/voice.wav", b"RIFF-spoken");
        let wav = stored(city.path(), "hall", b"RIFF\x24\x00\x00\x00WAVEfmt spoken");
        let flac = stored(city.path(), "hall", b"fLaC-stored");
        let tool = tool(city.path());
        let answered: Vec<(AxCode, String)> = [
            "vault/room/voice.wav",
            "hall/.sprawling/voice.wav",
            "hall/room/absent.wav",
            "hall/room/voice.flac",
            flac.as_str(),
            "hall/room/voice.wav",
            wav.as_str(),
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
                (AxCode::GateDenied, ACTION),
                (AxCode::GateDenied, ACTION),
                (AxCode::InvalidArgs, ACTION),
                (AxCode::InvalidArgs, "read a recording's container"),
                (AxCode::InvalidArgs, "read a recording's container"),
                (AxCode::ToolUnavailable, "transcribe a recording"),
                (AxCode::ToolUnavailable, "transcribe a recording"),
            ]
            .map(|(code, action)| (code, action.to_owned()))
        );
    }
}
