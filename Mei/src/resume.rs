// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Explicit, complete JSONL message recovery (`Mei/spec/Session.lean`).
//! Observation metadata is checked here and never sent as a message field.

use std::io::BufRead;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Result};

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    path: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        Code::Session,
        action,
        subject,
        "use a Mei JSONL whose first line is the initial user prompt and whose last assistant line has stop_reason=end_turn",
    )
    .with_path(path)
}

fn offending_value(value: Option<&Value>) -> String {
    value.map_or_else(|| "<missing>".to_owned(), Value::to_string)
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
    let mut last_entry = None;
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
        if value.get("type").and_then(Value::as_str) == Some("mei.obs") {
            continue;
        }
        let object = value.as_object().ok_or_else(|| {
            failure(
                "validate a Mei resume log line",
                "each line must be one message object",
                where_failed.clone(),
            )
            .with_value(value.to_string())
        })?;
        let role_value = object.get("role");
        let role = role_value.and_then(Value::as_str).ok_or_else(|| {
            failure(
                "validate a Mei resume log line",
                "role must be user or assistant",
                where_failed.clone(),
            )
            .with_value(offending_value(role_value))
        })?;
        if role != "user" && role != "assistant" {
            return Err(failure(
                "validate a Mei resume log line",
                "role must be user or assistant",
                where_failed,
            )
            .with_value(offending_value(role_value))
            .into());
        }
        let content_value = object.get("content");
        let content = content_value
            .filter(|content| content.is_string() || content.is_array())
            .ok_or_else(|| {
                failure(
                    "validate a Mei resume log line",
                    "content must be a string or an array",
                    where_failed.clone(),
                )
                .with_value(offending_value(content_value))
            })?;
        if messages.is_empty() && role != "user" {
            return Err(failure(
                "validate the initial Mei prompt",
                "the first line must contain the initial user prompt",
                where_failed,
            )
            .with_value(offending_value(role_value))
            .into());
        }
        messages.push(json!({"role": role, "content": content}));
        last_entry = Some(value);
    }
    let complete = last_entry.as_ref().is_some_and(|entry| {
        entry.get("role").and_then(Value::as_str) == Some("assistant")
            && entry.get("stop_reason").and_then(Value::as_str) == Some("end_turn")
    });
    if !complete {
        let offending = last_entry.as_ref().map_or_else(
            || "<empty log>".to_owned(),
            |entry| {
                if entry.get("role").and_then(Value::as_str) == Some("assistant") {
                    offending_value(entry.get("stop_reason"))
                } else {
                    offending_value(entry.get("role"))
                }
            },
        );
        return Err(failure(
            "validate the complete Mei session",
            "the log is empty or does not end with an assistant end_turn",
            path.display().to_string(),
        )
        .with_value(offending)
        .into());
    }
    Ok(messages)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn a_complete_log_preserves_messages_and_drops_observation_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"mei.obs\",\"kind\":\"usage\",\"usage\":{\"input_tokens\":1}}\n",
                "{\"role\":\"user\",\"content\":\"first\"}\n",
                "{\"type\":\"mei.obs\",\"kind\":\"pulse\"}\n",
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
    fn an_initial_multimodal_prompt_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multimodal.jsonl");
        let content = json!([
            {"type": "text", "text": "inspect"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "aW1hZ2U="}},
            {"type": "audio", "source": {"type": "base64", "media_type": "audio/wav", "data": "YXVkaW8="}},
            {"type": "video", "source": {"type": "url", "url": "https://example.invalid/video"}}
        ]);
        std::fs::write(
            &path,
            format!(
                "{}\n{}",
                json!({"role": "user", "content": content}),
                json!({"role": "assistant", "content": "done", "stop_reason": "end_turn"})
            ),
        )
        .unwrap();
        assert_eq!(
            read(&path).unwrap(),
            vec![
                json!({"role": "user", "content": content}),
                json!({"role": "assistant", "content": "done"})
            ]
        );
    }

    #[test]
    fn validation_failures_carry_role_content_and_stop_values() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes, expected) in [
            (
                "bad-role",
                "{\"role\":\"system\",\"content\":\"first\"}\n",
                "\"system\"",
            ),
            ("bad-content", "{\"role\":\"user\",\"content\":42}\n", "42"),
            (
                "bad-stop",
                concat!(
                    "{\"role\":\"user\",\"content\":\"first\"}\n",
                    "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"tool_use\"}\n"
                ),
                "\"tool_use\"",
            ),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            match read(&path).unwrap_err() {
                Error::Diagnostic(diagnostic) => {
                    assert_eq!(diagnostic.value.as_deref(), Some(expected), "{name}");
                }
                error @ Error::Body(_) => panic!("{name}: unexpected raw body: {error}"),
            }
        }
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
            let expected_path = if matches!(name, "empty" | "partial") {
                path.display().to_string()
            } else {
                format!("{}:1", path.display())
            };
            match read(&path).unwrap_err() {
                Error::Diagnostic(diagnostic) => {
                    assert_eq!(diagnostic.code, Code::Session, "{name}");
                    assert_eq!(
                        diagnostic.path.as_deref(),
                        Some(expected_path.as_str()),
                        "{name}"
                    );
                }
                error @ Error::Body(_) => panic!("{name}: unexpected raw body: {error}"),
            }
        }
    }
}
