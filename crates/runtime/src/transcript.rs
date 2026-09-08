// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one run actually saw, written beside its room when it freezes.
//!
//! One `ChatMessage` per line, in the serde form the wire uses, so a
//! line number `search` reports is a message index `read` continues
//! from. The frozen prefix is not here: it is the half of every request
//! that never changes, and the ledger's `prompt_assembled` already
//! holds its hash.
//!
//! **This is not the ledger, and it must not point at it.** The ledger
//! lives under the reserved subtree `read` refuses, and it is one chain
//! for the whole city — handing it to one resident hands over a
//! confidential building's events as well. A transcript is one run's,
//! written in that run's room, and residents of the same building may
//! read it; a confidential building is protected by the isolation it
//! already has.
//!
//! Three steps in a fixed order: scanned for credentials, pinned into
//! the CAS, then materialised. Scanned before pinned, because a key that
//! reached the content store cannot be taken back out.

use std::path::Path;

use kernel::{Address, AxCode, AxError, Locator, RunId};
use memory::Cas;
use serde_json::Value;

use crate::redact;
use crate::window::Window;

/// The file a transcript is written to, beside the room's own documents.
const TRANSCRIPT_EXT: &str = "jsonl";

/// The messages one run's model saw, one JSON line each, already
/// scanned. Private fields: a transcript that skipped the scan cannot
/// be assembled a line at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    run: RunId,
    lines: Vec<String>,
    redacted: u32,
}

/// Where a transcript landed, and the way back to its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRecord {
    pub address: Address,
    pub original: Locator,
    pub redacted: u32,
}

impl Transcript {
    /// Renders and scans every message of the window.
    ///
    /// # Errors
    /// Refuses a message that will not serialise, which is a defect in
    /// the wire types rather than in the conversation.
    pub fn of(run: RunId, window: &Window) -> Result<Transcript, AxError> {
        let mut lines = Vec::with_capacity(window.messages().len());
        let mut redacted: u32 = 0;
        for message in window.messages() {
            let value = serde_json::to_value(message).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    err.to_string(),
                )
            })?;
            let Value::Object(map) = value else {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    "a message is an object",
                ));
            };
            let (scanned, hits) = redact::redact(&map);
            redacted = redacted.saturating_add(hits);
            let line = serde_json::to_string(&Value::Object(scanned)).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    err.to_string(),
                )
            })?;
            lines.push(line);
        }
        Ok(Transcript {
            run,
            lines,
            redacted,
        })
    }

    /// `<room>/<run-id>.jsonl`. Pure, so a handoff can name the address
    /// before the run it belongs to has taken a turn.
    ///
    /// # Errors
    /// Propagates a room whose address cannot take another segment.
    pub fn address(room: &Address, run: RunId) -> Result<Address, AxError> {
        Address::parse(&format!("{}/{run}.{TRANSCRIPT_EXT}", room.as_str()))
    }

    pub fn run(&self) -> RunId {
        self.run
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// How many secret-shaped spans the scan replaced.
    pub fn redacted(&self) -> u32 {
        self.redacted
    }

    /// The bytes of the file: every line newline-terminated.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for line in &self.lines {
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
        out
    }

    /// Pins the transcript in the store, then writes it read-only at
    /// its address under `city_root`.
    ///
    /// Writing the same transcript twice is writing it once: the store
    /// deduplicates by content, and a file already at the address is
    /// left as it is.
    ///
    /// # Errors
    /// Propagates a store that will not take the bytes and a room that
    /// will not take the file, each naming the path.
    pub fn materialise(
        &self,
        cas: &mut Cas,
        city_root: &Path,
        room: &Address,
    ) -> Result<TranscriptRecord, AxError> {
        let bytes = self.bytes();
        let hash = cas.put(&bytes).map_err(memory::MemoryError::into_ax)?;
        let original = Locator::parse(&format!("cas:b3-{hash}"))?;
        let address = Transcript::address(room, self.run)?;
        let path = city_root.join(address.as_str());
        let io = |err: std::io::Error| {
            AxError::failure(
                AxCode::StorageFatal,
                "materialise a transcript",
                format!("{}: {err}", path.display()),
            )
            .with_recovery("check the room is writable; the transcript is still in the store")
        };
        if !path.exists() {
            std::fs::write(&path, &bytes).map_err(io)?;
            let mut perms = std::fs::metadata(&path).map_err(io)?.permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(&path, perms).map_err(io)?;
        }
        Ok(TranscriptRecord {
            address,
            original,
            redacted: self.redacted,
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
    use kernel::ContentBlock;

    fn run() -> RunId {
        RunId::parse("0198f6a2-7c4a-7bbb-9d1e-000000000009").unwrap()
    }

    fn window_with_a_call() -> Window {
        let mut window = Window::new();
        window.push_task_lines("look", "one call", crate::window::Opening::FromJob);
        window.push_assistant(vec![ContentBlock::ToolUse {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("status").unwrap(),
            input: kernel::Payload::empty(),
        }]);
        window.push_tool_results(vec![ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: "{\"ok\":true}".to_owned(),
            is_error: false,
            attachments: Vec::new(),
        }]);
        window
    }

    #[test]
    fn one_line_per_message_in_the_wire_shape() {
        let transcript = Transcript::of(run(), &window_with_a_call()).unwrap();
        assert_eq!(transcript.lines().len(), 3);
        let first: Value = serde_json::from_str(&transcript.lines()[0]).unwrap();
        assert_eq!(first["role"], "user");
        assert!(transcript.lines()[1].contains("tu_1"));
        assert!(transcript.lines()[2].contains("tu_1"));
        assert_eq!(transcript.redacted(), 0);
    }

    #[test]
    fn a_key_the_model_saw_does_not_reach_the_file() {
        let key = format!("sk-ant-api03-{}", "A".repeat(80));
        let mut window = Window::new();
        window.push_tool_results(vec![ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: format!("token={key}"),
            is_error: false,
            attachments: Vec::new(),
        }]);
        let transcript = Transcript::of(run(), &window).unwrap();
        assert!(
            !transcript.lines()[0].contains(&key),
            "{:?}",
            transcript.lines()
        );
        assert!(transcript.redacted() >= 1);
    }

    #[test]
    fn the_address_is_the_room_and_the_run_and_never_the_reserved_subtree() {
        let room = Address::parse("lab/room1").unwrap();
        let address = Transcript::address(&room, run()).unwrap();
        assert_eq!(
            address.as_str(),
            "lab/room1/0198f6a2-7c4a-7bbb-9d1e-000000000009.jsonl"
        );
        assert!(!address.is_reserved());
    }

    #[test]
    fn materialising_pins_then_writes_read_only_and_repeats_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
        let room = Address::parse("lab/room1").unwrap();
        std::fs::create_dir_all(dir.path().join("lab/room1")).unwrap();
        let transcript = Transcript::of(run(), &window_with_a_call()).unwrap();
        let record = transcript.materialise(&mut cas, dir.path(), &room).unwrap();
        let path = dir.path().join(record.address.as_str());
        assert_eq!(std::fs::read(&path).unwrap(), transcript.bytes());
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
        assert!(record.original.to_string().starts_with("cas:b3-"));
        let again = transcript.materialise(&mut cas, dir.path(), &room).unwrap();
        assert_eq!(again, record);
    }
}
