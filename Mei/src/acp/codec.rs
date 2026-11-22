// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::prompt_text;
use crate::error::{Code, Diagnostic, Error, Result};

pub(super) const PROTOCOL_VERSION: u64 = 1;

pub(super) fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": {}
        },
        "agentInfo": {
            "name": "mei",
            "title": "Mei",
            "version": env!("CARGO_PKG_VERSION")
        },
        "authMethods": []
    })
}

pub(super) fn open_session(
    params: &Value,
    next: &mut u64,
) -> std::result::Result<(String, PathBuf), String> {
    let cwd = params
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or_else(|| "session/new requires a UTF-8 cwd string".to_owned())?;
    if cwd.is_empty() {
        return Err("session/new cwd must not be empty".to_owned());
    }
    let path = PathBuf::from(cwd);
    if !path.is_absolute() {
        return Err("session/new cwd must be an absolute path".to_owned());
    }
    if !path.is_dir() {
        return Err(format!(
            "session/new cwd is not a directory: {}",
            path.display()
        ));
    }
    let id = *next;
    *next = next
        .checked_add(1)
        .ok_or_else(|| "too many ACP sessions in one process; restart mei --acp".to_owned())?;
    Ok((format!("mei_{id}"), path))
}

pub(super) fn prompt_params(
    params: &Value,
    session_id: &str,
) -> std::result::Result<String, String> {
    let asked = params
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "session/prompt requires sessionId".to_owned())?;
    if asked != session_id {
        return Err(format!("unknown sessionId `{asked}`"));
    }
    let prompt = params
        .get("prompt")
        .and_then(Value::as_array)
        .ok_or_else(|| "session/prompt requires a prompt array".to_owned())?;
    prompt_text(prompt).ok_or_else(|| {
        "session/prompt needs at least one non-empty text or resource_link block".to_owned()
    })
}

pub(super) fn write_result(output: &mut impl Write, id: &Value, result: Value) -> Result<()> {
    write_message(
        output,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

pub(super) fn write_error(
    output: &mut impl Write,
    id: &Value,
    code: i64,
    message: String,
) -> Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    )
}

pub(super) fn write_update(output: &mut impl Write, session_id: &str, update: Value) -> Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": { "sessionId": session_id, "update": update }
        }),
    )
}

fn write_message(output: &mut impl Write, message: &Value) -> Result<()> {
    let mut line = serde_json::to_string(message).map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "encode an ACP message",
            err.to_string(),
            "keep ACP payloads within JSON object limits",
        ))
    })?;
    line.push('\n');
    output.write_all(line.as_bytes()).map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "write an ACP message",
            err.to_string(),
            "keep the client stdout pipe open for JSON-RPC replies",
        ))
    })?;
    output.flush().map_err(|err| {
        Error::Diagnostic(Diagnostic::new(
            Code::Protocol,
            "flush ACP stdout",
            err.to_string(),
            "keep the client stdout pipe open for JSON-RPC replies",
        ))
    })
}
