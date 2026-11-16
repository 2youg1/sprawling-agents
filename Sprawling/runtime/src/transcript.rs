// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::TRANSCRIPT_EXT;
use kernel::{Address, AxCode, AxError, Locator, RunId};
use serde_json::Value;
use storage::Cas;

use crate::conversation::Conversation;
use crate::redact;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    run: RunId,
    lines: Vec<String>,
    redacted: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRecord {
    pub address: Address,
    pub original: Locator,
    pub redacted: u32,
}

impl Transcript {

    pub fn of(run: RunId, conversation: &Conversation) -> Result<Transcript, AxError> {
        let mut lines = Vec::with_capacity(conversation.messages().len());
        let mut redacted: u32 = 0;
        for message in conversation.messages() {
            let value = serde_json::to_value(message).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    err.to_string(),
                )
                .with_recovery(
                    "report this against runtime::transcript: a conversation message is text \
                     and content blocks, and JSON refuses neither",
                )
            })?;
            let Value::Object(map) = value else {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    "a message is an object",
                )
                .with_recovery(
                    "report this against runtime::transcript: a conversation message encodes \
                     as a JSON object and this one encoded as something else",
                ));
            };
            let (scanned, hits) = redact::redact(map);
            redacted = redacted.saturating_add(hits);
            let line = serde_json::to_string(&Value::Object(scanned)).map_err(|err| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "render a transcript line",
                    err.to_string(),
                )
                .with_recovery(
                    "report this against runtime::transcript: redaction returns the \
                     same JSON object with some strings replaced",
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

    pub fn address(room: &Address, run: RunId) -> Result<Address, AxError> {
        Address::parse(&format!("{}/{run}.{TRANSCRIPT_EXT}", room.as_str()))
    }

    pub fn run(&self) -> RunId {
        self.run
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn redacted(&self) -> u32 {
        self.redacted
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for line in &self.lines {
            out.extend_from_slice(line.as_bytes());
            out.push(b'\n');
        }
        out
    }

    pub fn materialise(
        &self,
        cas: &mut Cas,
        city_root: &Path,
        room: &Address,
    ) -> Result<TranscriptRecord, AxError> {
        let bytes = self.bytes();
        let origin = storage::BlockOrigin {
            run: self.run,
            building: room.clone(),
        };
        let hash = cas
            .put_for(&bytes, &origin)
            .map_err(storage::StorageError::into_ax)?;
        let original = Locator::cas(hash);
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

    fn window_with_a_call() -> Conversation {
        let mut conversation = Conversation::new();
        conversation.push_task_lines(
            "look",
            "one call",
            crate::conversation::Opening::FromJob,
            "the User",
        );
        conversation.push_assistant(vec![ContentBlock::ToolUse {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("status").unwrap(),
            input: kernel::Payload::empty(),
        }]);
        conversation.push_tool_results(vec![ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: "{\"ok\":true}".to_owned(),
            is_error: false,
            attachments: Vec::new(),
        }]);
        conversation
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
        let mut conversation = Conversation::new();
        conversation.push_tool_results(vec![ContentBlock::ToolResult {
            tool_use_id: "tu_1".to_owned(),
            content: format!("token={key}"),
            is_error: false,
            attachments: Vec::new(),
        }]);
        let transcript = Transcript::of(run(), &conversation).unwrap();
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
