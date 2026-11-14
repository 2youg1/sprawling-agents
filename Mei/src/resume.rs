// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Explicit, complete JSONL message recovery (`Mei/spec/Session.lean`).
//! Observation metadata is checked here and never sent as a message field.

use std::io::BufRead;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result};

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    path: impl Into<String>,
) -> Error {
    Diagnostic::new(
        Code::Session,
        action,
        subject,
        "use a Mei JSONL whose first line is the initial user prompt and whose last assistant line has stop_reason=end_turn",
    )
    .with_path(path)
    .into()
}

/// Reads role/content in file order from a complete Mei observation log.
/// The first prompt and final `end_turn` keep partial or foreign JSONL out
/// of the request; no model or Python process has started at this point.
pub fn read(path: &Path) -> Result<Vec<Value>> {
    let file = std::fs::File::open(path).map_err(|err| {
        failure(
            "open the Mei resume log",
            err.to_string(),
            path.display().to_string(),
        )
    })?;
    let mut messages = Vec::new();
    let mut final_stop = None;
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line_number = index.saturating_add(1);
        let where_failed = format!("{}:{line_number}", path.display());
        let line = line.map_err(|err| {
            failure(
                "read the Mei resume log",
                err.to_string(),
                where_failed.clone(),
            )
        })?;
        let value: Value = serde_json::from_str(&line).map_err(|err| {
            failure(
                "parse a Mei resume log line",
                err.to_string(),
                where_failed.clone(),
            )
        })?;
        let object = value.as_object().ok_or_else(|| {
            failure(
                "validate a Mei resume log line",
                "each line must be one message object",
                where_failed.clone(),
            )
        })?;
        let role = object.get("role").and_then(Value::as_str).ok_or_else(|| {
            failure(
                "validate a Mei resume log line",
                "role must be user or assistant",
                where_failed.clone(),
            )
        })?;
        if role != "user" && role != "assistant" {
            return Err(failure(
                "validate a Mei resume log line",
                "role must be user or assistant",
                where_failed,
            ));
        }
        let content = object
            .get("content")
            .filter(|content| content.is_string() || content.is_array())
            .ok_or_else(|| {
                failure(
                    "validate a Mei resume log line",
                    "content must be a string or an array",
                    where_failed.clone(),
                )
            })?;
        if messages.is_empty() && (role != "user" || !content.is_string()) {
            return Err(failure(
                "validate the initial Mei prompt",
                "the first line must contain the initial user prompt as a string",
                where_failed,
            ));
        }
        final_stop = if role == "assistant" {
            object
                .get("stop_reason")
                .and_then(Value::as_str)
                .map(str::to_owned)
        } else {
            None
        };
        messages.push(json!({"role": role, "content": content}));
    }
    let complete = messages
        .last()
        .is_some_and(|message| message["role"] == "assistant")
        && final_stop.as_deref() == Some("end_turn");
    if !complete {
        return Err(failure(
            "validate the complete Mei session",
            "the log is empty or does not end with an assistant end_turn",
            path.display().to_string(),
        ));
    }
    Ok(messages)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_complete_log_preserves_messages_and_drops_observation_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"role\":\"user\",\"content\":\"first\"}\n",
                "{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"done\"}],\"stop_reason\":\"end_turn\"}\n"
            ),
        )
        .unwrap();
        assert_eq!(
            read(&path).unwrap(),
            vec![
                json!({"role": "user", "content": "first"}),
                json!({"role": "assistant", "content": [{"type": "text", "text": "done"}]})
            ]
        );
    }

    #[test]
    fn empty_partial_and_foreign_logs_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("empty", ""),
            ("partial", "{\"role\":\"user\",\"content\":\"first\"}\n"),
            (
                "foreign",
                "{\"role\":\"assistant\",\"content\":\"old\",\"stop_reason\":\"end_turn\"}\n",
            ),
            ("bad-json", "not json\n"),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            let error = read(&path).unwrap_err().to_string();
            assert!(error.contains("E_MEI_SESSION"), "{name}: {error}");
            assert!(error.contains(&path.display().to_string()), "{error}");
        }
    }
}
