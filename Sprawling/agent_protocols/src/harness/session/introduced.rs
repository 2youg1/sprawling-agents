// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

pub(super) const PROTOCOL_VERSION: u64 = 1;

pub(super) const AUTH_REQUIRED: i64 = -32_000;

const NEVER_STARTED: &str = "claude-ai-login";

pub(super) fn initialize_params() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "clientInfo": { "name": "sprawling", "version": env!("CARGO_PKG_VERSION") },
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
            "auth": { "terminal": true }
        }
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Introduced {
    pub version: Option<String>,

    pub auth_methods: Vec<AuthMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthMethod {
    pub id: String,
    pub name: String,
    pub kind: LoginKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginKind {
    Agent,

    Terminal,
}

pub(super) fn introduced(name: &str, answer: &Value) -> Result<Introduced, AxError> {
    let spoken = answer.get("protocolVersion").and_then(Value::as_u64);
    if spoken != Some(PROTOCOL_VERSION) {
        let said = spoken.map_or_else(|| "no version".to_owned(), |v| format!("version {v}"));
        return Err(AxError::failure(
            AxCode::WireMismatch,
            format!("initialize {name}"),
            format!("the agent speaks ACP {said}; this city speaks version {PROTOCOL_VERSION}"),
        )
        .with_recovery("install a release of the agent that speaks ACP version 1"));
    }
    let methods = answer
        .get("authMethods")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut auth_methods = Vec::new();
    for method in methods {
        let text = |key: &str| method.get(key).and_then(Value::as_str).unwrap_or_default();
        let kind = match method.get("type").and_then(Value::as_str) {
            None | Some("agent") => LoginKind::Agent,
            Some("terminal") => LoginKind::Terminal,
            Some(other) => {
                return Err(AxError::failure(
                    AxCode::WireMismatch,
                    format!("initialize {name}"),
                    format!("a login of type `{other}`"),
                )
                .with_recovery("install a release of the agent that speaks ACP version 1"));
            }
        };
        if text("id") != NEVER_STARTED {
            auth_methods.push(AuthMethod {
                id: text("id").to_owned(),
                name: text("name").to_owned(),
                kind,
            });
        }
    }
    Ok(Introduced {
        version: answer
            .get("agentInfo")
            .and_then(|info| info.get("version"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        auth_methods,
    })
}

pub(super) fn auth_required(name: &str, method: &str, offered: &[AuthMethod]) -> AxError {
    let names = offered
        .iter()
        .map(|method| method.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let recovery = if offered.is_empty() {
        format!("sign in inside {name} itself, then dispatch again")
    } else {
        format!("sign in to {name} with one of: {names}; then dispatch again")
    };
    AxError::failure(
        AxCode::AuthRequired,
        format!("ask {name} to {method}"),
        format!("{name} asks for a login"),
    )
    .with_nearby(offered.iter().map(|method| method.id.clone()).collect())
    .with_recovery(recovery)
}
