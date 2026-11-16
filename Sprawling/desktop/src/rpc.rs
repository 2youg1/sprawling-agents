// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::refusal::{Refusal, RefusalCode};
use serde_json::{Value, json};

pub(crate) use agent_protocols::PROTOCOL_VERSION;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) id: Option<Value>,
    pub(crate) method: String,

    pub(crate) params: Value,
}

pub(crate) fn read(line: &str) -> Result<Request, Refusal> {
    let value: Value = serde_json::from_str(line).map_err(|err| {
        Refusal::new(
            RefusalCode::WireMismatch,
            "read a request",
            err.to_string(),
            "send one JSON-RPC 2.0 object per line",
        )
    })?;
    let object = value.as_object().ok_or_else(|| {
        Refusal::new(
            RefusalCode::WireMismatch,
            "read a request",
            "the line is not a JSON object",
            "send one JSON-RPC 2.0 object per line",
        )
    })?;
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Refusal::new(
                RefusalCode::WireMismatch,
                "read a request",
                "the message names no method",
                "every request carries a `method`",
            )
        })?
        .to_owned();
    let params = match object.get("params") {
        None | Some(Value::Null) => json!({}),
        Some(given) if given.is_object() => given.clone(),
        Some(_other) => {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "read a request",
                format!("`{method}` was sent params that are not an object"),
                "send `params` as an object, as every tool schema here describes",
            ));
        }
    };
    Ok(Request {
        id: object.get("id").cloned(),
        method,
        params,
    })
}

pub(crate) fn result_line(id: &Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

pub(crate) fn error_line(id: Option<&Value>, refusal: &Refusal) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned().unwrap_or(Value::Null),
        "error": refusal.as_error(),
    })
    .to_string()
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

    #[test]
    fn every_line_this_server_writes_is_one_line() {
        let answer = result_line(&json!(7), json!({ "tools": [{ "name": "desktop.act" }] }));
        assert!(!answer.contains('\n'));
        assert_eq!(
            answer,
            "{\"id\":7,\"jsonrpc\":\"2.0\",\"result\":{\"tools\":[{\"name\":\"desktop.act\"}]}}"
        );
        let refused = error_line(
            None,
            &Refusal::new(
                RefusalCode::ToolUnknown,
                "call a tool",
                "nope",
                "read the list",
            ),
        );
        assert!(!refused.contains('\n'));
        assert!(refused.contains("\"id\":null"));
    }

    #[test]
    fn an_id_comes_back_untouched_and_a_notification_has_none() {
        let request = read("{\"jsonrpc\":\"2.0\",\"id\":\"a-1\",\"method\":\"ping\"}").unwrap();
        assert_eq!(request.id, Some(json!("a-1")));
        assert_eq!(request.params, json!({}));
        let notification =
            read("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}")
                .unwrap();
        assert_eq!(notification.id, None);
        assert_eq!(notification.method, "notifications/initialized");
    }

    #[test]
    fn a_line_this_version_cannot_read_is_refused_rather_than_guessed() {
        for line in ["not json", "[]", "{\"jsonrpc\":\"2.0\",\"id\":1}"] {
            let refusal = read(line).unwrap_err();
            assert_eq!(
                refusal.as_error()["data"]["code"],
                "E_WIRE_MISMATCH",
                "{line}"
            );
        }
        let odd = read("{\"method\":\"tools/call\",\"id\":1,\"params\":[]}").unwrap_err();
        let error = odd.as_error();
        assert_eq!(error["data"]["code"], "E_INVALID_ARGS");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("object")
        );
    }
}
