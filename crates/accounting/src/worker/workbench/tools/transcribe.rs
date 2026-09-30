// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `transcribe`: a recording in this run's own building, turned into text
//! by the endpoint the person chose to transcribe (sprawling-SPEC.md
//! 8-131).
//!
//! Whether the tool exists is the book's answer to the question the
//! composer's microphone asks, `select(ModelTag::Transcribe, policy)`,
//! read under the rules of the building the run stands in. A run in a
//! building the book gives no transcription endpoint is not offered a
//! tool that could only fail.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::{
    Address, AxCode, AxError, CostTier, Effect, Payload, RenderIntent, Temporal, Tool, ToolCall,
    ToolMeta, ToolName, ToolOutcome,
};
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
        TranscribeTool::new(&site.write_root, site.building.addr().clone(), transcriber).map(Some)
    }
}

/// The tool: where the run reads, which building it may read a
/// recording from, and the facility that turns one into text.
pub(super) struct TranscribeTool {
    /// The tree the run reads in: the city, or its own worktree under
    /// review.
    root: PathBuf,
    building: Address,
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
        root: &Path,
        building: Address,
        transcriber: gateway::Transcriber,
    ) -> Result<TranscribeTool, AxError> {
        let params = json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "a recording in your own building, relative to the city \
                                    root, ending in .webm, .ogg, .mp3, .mp4 or .wav",
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
            root: root.to_path_buf(),
            building,
            transcriber: Mutex::new(transcriber),
            meta: ToolMeta {
                name: ToolName::parse(ACTION)?,
                disclosure: "Turn a recording of speech in your own building into text, \
                             through the transcription endpoint this city was given. The \
                             recording is sent to that endpoint."
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

    /// The recording `asked` names, once the path is judged.
    ///
    /// Every judgement is an existing authority's: the address grammar,
    /// the building it lies in, the reserved subtree, and the alias rule,
    /// which refuses a link anywhere on the way rather than following it.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a path that does not parse, a container this
    /// city cannot send, and a file that is not there; `E_GATE_DENIED`
    /// for a path outside this building, in a reserved subtree, or behind
    /// a link; whatever reading the recording refuses.
    fn recording(&self, asked: &str) -> Result<gateway::Recording, AxError> {
        let addr = Address::parse(asked).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                ACTION,
                format!("{asked}: {}", err.subject()),
            )
            .with_recovery(
                "pass a city-relative path with no `..`, no leading slash and no empty segment",
            )
        })?;
        if !addr.is_within(&self.building) || addr.is_reserved() {
            return Err(AxError::failure(
                AxCode::GateDenied,
                ACTION,
                format!(
                    "{asked} is not a file of {}, outside its reserved subtree",
                    self.building.as_str()
                ),
            )
            .with_recovery(format!(
                "a recording is transcribed from the building whose rules chose the endpoint \
                 it is sent to; copy it into {} first",
                self.building.as_str()
            )));
        }
        let kind = gateway::AudioType::of_file_name(asked)?;
        let path = addr
            .as_str()
            .split('/')
            .fold(self.root.clone(), |path, segment| path.join(segment));
        let cleared = storage::WriteTarget::within(ACTION, &self.root, &path)
            .map_err(storage::StorageError::into_ax)?;
        let file = std::fs::File::open(cleared.as_path()).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AxError::failure(AxCode::InvalidArgs, ACTION, format!("{asked} is not there"))
                    .with_recovery("name a recording that exists; `search` finds files by name")
            } else {
                AxError::failure(AxCode::StorageFatal, ACTION, format!("{asked}: {err}"))
                    .with_recovery("a person has to make the file readable")
            }
        })?;
        gateway::Recording::read_from(file, kind)
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
                .with_recovery("pass one string: the city-relative path of a recording")
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
    use super::*;

    /// A tool over a city in a temporary directory, working in `lab`,
    /// with no endpoint behind it: every case here is refused before
    /// anything would be sent.
    fn tool(root: &Path) -> TranscribeTool {
        TranscribeTool::new(
            root,
            Address::parse("lab").unwrap(),
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

    fn refused(root: &Path, path: &str) -> AxError {
        tool(root).invoke(&call(path)).unwrap_err()
    }

    /// The one building whose rules chose the endpoint is the one a
    /// recording may come from, and its governance is never read.
    #[test]
    fn a_recording_outside_this_building_or_in_its_reserved_subtree_is_refused() {
        let city = tempfile::tempdir().unwrap();
        for elsewhere in ["hall/dropped/0/voice.wav", "lab/.sprawling/voice.wav"] {
            let err = refused(city.path(), elsewhere);
            assert_eq!(*err.code(), AxCode::GateDenied, "{elsewhere}: {err:?}");
            assert!(err.recovery().contains("lab"), "{}", err.recovery());
        }
        let absolute = refused(city.path(), "/lab/voice.wav");
        assert_eq!(*absolute.code(), AxCode::InvalidArgs, "{absolute:?}");
    }

    /// A container this city cannot send is refused with the refusal the
    /// gateway writes, before the file is opened.
    #[test]
    fn a_recording_this_city_cannot_send_is_refused_with_the_ones_it_can() {
        let city = tempfile::tempdir().unwrap();
        let err = refused(city.path(), "lab/lead/voice.flac");
        assert_eq!(*err.code(), AxCode::InvalidArgs, "{err:?}");
        assert!(err.recovery().contains(".wav"), "{}", err.recovery());
        let missing = refused(city.path(), "lab/lead/voice.wav");
        assert_eq!(*missing.code(), AxCode::InvalidArgs, "{missing:?}");
        assert!(missing.subject().contains("not there"), "{missing:?}");
    }

    /// A recording that reads is handed to the facility, which here has
    /// no endpoint and says so by name.
    #[test]
    fn a_recording_that_reads_reaches_the_facility() {
        let city = tempfile::tempdir().unwrap();
        let room = city.path().join("lab").join("lead");
        std::fs::create_dir_all(&room).unwrap();
        std::fs::write(room.join("voice.wav"), b"RIFF-spoken").unwrap();
        let err = refused(city.path(), "lab/lead/voice.wav");
        assert_eq!(*err.code(), AxCode::ToolUnavailable, "{err:?}");
        assert_eq!(err.action(), "transcribe a recording");
    }
}
