// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result};

const PERSON: &str = "user";

#[must_use]
pub fn render(text: &str) -> String {
    format!("{PERSON}: {text}")
}

fn join_steer(body: &mut String, line: &str) {
    if body.is_empty() {
        *body = line.to_owned();
    } else if body.ends_with('\n') {
        body.push_str(line);
    } else {
        body.push('\n');
        body.push_str(line);
    }
}

pub fn append_to_content(content: &mut Value, text: &str) {
    let line = render(text);
    match content {
        Value::String(body) => join_steer(body, &line),
        Value::Array(blocks) => {
            if let Some(Value::Object(block)) = blocks.iter_mut().rev().find(|block| {
                block.get("type").and_then(Value::as_str) == Some("text")
                    && block.get("text").and_then(Value::as_str).is_some()
            }) {
                let Some(Value::String(body)) = block.get_mut("text") else {
                    return;
                };
                join_steer(body, &line);
                return;
            }
            blocks.push(json!({"type": "text", "text": line}));
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Object(_) => {
            *content = Value::String(line);
        }
    }
}

pub fn land(results: &mut [Value], steers: &[String]) {
    let Some(last) = results.last_mut() else {
        return;
    };
    let Some(content) = last.get_mut("content") else {
        return;
    };
    for text in steers {
        if text.is_empty() {
            continue;
        }
        append_to_content(content, text);
    }
}

pub struct Intake {
    path: PathBuf,
    offset: u64,
    pending: Vec<u8>,
}

impl Intake {
    pub fn open(path: impl AsRef<Path>) -> Result<Intake> {
        Ok(Intake {
            path: path.as_ref().to_path_buf(),
            offset: 0,
            pending: Vec::new(),
        })
    }

    pub fn drain(&mut self) -> Result<Vec<String>> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(self.io("open the Mei steer file", err)),
        };
        file.seek(SeekFrom::Start(self.offset))
            .map_err(|err| self.io("seek the Mei steer file", err))?;
        let mut chunk = Vec::new();
        file.read_to_end(&mut chunk)
            .map_err(|err| self.io("read the Mei steer file", err))?;
        let read = u64::try_from(chunk.len()).map_err(|_| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Config,
                    "measure a Mei steer append",
                    "the newly appended bytes do not fit in u64",
                    "keep each steer line small enough for the Face JSONL contract",
                )
                .with_path(self.path.display().to_string()),
            )
        })?;
        self.offset = self.offset.saturating_add(read);
        self.pending.extend_from_slice(&chunk);
        let mut steers = Vec::new();
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line = self.pending.drain(..=end).collect::<Vec<u8>>();
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                continue;
            }
            let text = std::str::from_utf8(&line).map_err(|err| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Config,
                        "decode a Mei steer line",
                        err.to_string(),
                        "append one UTF-8 JSON object per line shaped as {\"text\":\"…\"}",
                    )
                    .with_path(self.path.display().to_string()),
                )
            })?;
            steers.push(self.parse_line(text)?);
        }
        Ok(steers)
    }

    fn parse_line(&self, line: &str) -> Result<String> {
        let value: Value = serde_json::from_str(line).map_err(|err| {
            Error::Diagnostic(
                Diagnostic::new(
                    Code::Config,
                    "parse a Mei steer line",
                    err.to_string(),
                    "append one JSON object per line shaped as {\"text\":\"…\"}",
                )
                .with_path(self.path.display().to_string())
                .with_value(line.to_owned()),
            )
        })?;
        value
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                Error::Diagnostic(
                    Diagnostic::new(
                        Code::Config,
                        "read a Mei steer line",
                        "text is missing or is not a string",
                        "append one JSON object per line shaped as {\"text\":\"…\"}",
                    )
                    .with_path(self.path.display().to_string())
                    .with_value(line.to_owned()),
                )
            })
    }

    fn io(&self, action: &str, err: std::io::Error) -> Error {
        Error::Diagnostic(
            Diagnostic::new(
                Code::Config,
                action,
                err.to_string(),
                "keep MEI_STEER_PATH pointing at a readable JSONL file the Face appends",
            )
            .with_path(self.path.display().to_string()),
        )
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
    use std::io::Write as _;

    #[test]
    fn string_content_grows_with_a_user_prefix() {
        let mut content = Value::String("40\n".to_owned());
        append_to_content(&mut content, "narrow the search");
        assert_eq!(content, Value::String("40\nuser: narrow the search".to_owned()));
    }

    #[test]
    fn image_blocks_keep_order_and_grow_the_text() {
        let mut content = json!([
            {"type": "text", "text": "opened"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "QQ=="}}
        ]);
        append_to_content(&mut content, "look again");
        assert_eq!(
            content,
            json!([
                {"type": "text", "text": "opened\nuser: look again"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "QQ=="}}
            ])
        );
    }

    #[test]
    fn land_only_touches_the_latest_result() {
        let mut results = vec![
            json!({"type": "tool_result", "tool_use_id": "a", "content": "1\n"}),
            json!({"type": "tool_result", "tool_use_id": "b", "content": "2\n"}),
        ];
        land(&mut results, &["prefer metres".to_owned()]);
        assert_eq!(results[0]["content"], "1\n");
        assert_eq!(results[1]["content"], "2\nuser: prefer metres");
    }

    #[test]
    fn intake_drains_complete_lines_and_waits_for_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("steer.jsonl");
        let mut intake = Intake::open(&path).unwrap();
        assert!(intake.drain().unwrap().is_empty());
        let mut file = File::create(&path).unwrap();
        write!(file, "{{\"text\":\"one\"}}\n{{\"text\":\"two\"}}\n{{\"text\":\"part").unwrap();
        file.flush().unwrap();
        assert_eq!(intake.drain().unwrap(), ["one", "two"]);
        writeln!(file, "ial\"}}").unwrap();
        file.flush().unwrap();
        assert_eq!(intake.drain().unwrap(), ["partial"]);
    }

    #[test]
    fn intake_waits_when_a_multibyte_character_is_split() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("steer.jsonl");
        let mut intake = Intake::open(&path).unwrap();
        let mut file = File::create(&path).unwrap();
        let line = "{\"text\":\"窄搜索\"}\n";
        let bytes = line.as_bytes();
        let split = bytes
            .iter()
            .position(|byte| *byte == 0xE7)
            .and_then(|start| start.checked_add(1))
            .expect("narrow starts with a three-byte character");
        file.write_all(&bytes[..split]).unwrap();
        file.flush().unwrap();
        assert!(intake.drain().unwrap().is_empty());
        file.write_all(&bytes[split..]).unwrap();
        file.flush().unwrap();
        assert_eq!(intake.drain().unwrap(), ["窄搜索"]);
    }
}
