// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::reception::{Admission, Door, decide_admission};

use super::config::ShellState;
use super::config::refusal_text;

pub(crate) async fn accept_acp(State(state): State<Arc<ShellState>>, body: Bytes) -> Response {
    let Ok(request) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send {\"token\":..,\"addr\":..,\"task\":..,\"goal\":..}",
        )
            .into_response();
    };

    let offered = match state.door.pairing(
        &state.face,
        request.get("token").and_then(serde_json::Value::as_str),
    ) {
        Ok(offered) => offered,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, refusal_text(&err)).into_response(),
    };
    let pairing = match decide_admission(Door::Acp, offered) {
        Admission::Admit(pairing) => pairing,
        Admission::Refuse(err) => {
            return (StatusCode::FORBIDDEN, refusal_text(&err)).into_response();
        }
    };
    match (state.acp)(&request, pairing) {
        Ok(progress) => (StatusCode::ACCEPTED, Json(progress)).into_response(),
        Err(err) => (StatusCode::FORBIDDEN, refusal_text(&err)).into_response(),
    }
}

pub(crate) async fn accept_recording(
    State(state): State<Arc<ShellState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(container) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|container| !container.is_empty())
    else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send the recording with the content-type it was recorded in",
        )
            .into_response();
    };
    match (state.transcribe_sink)(body.to_vec(), container.to_owned()) {
        Ok(text) => (StatusCode::OK, text).into_response(),
        Err(err) => (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response(),
    }
}

pub(crate) async fn accept_drop(
    State(state): State<Arc<ShellState>>,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let Some(name) = params.get("name") else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send the file's name as `?name=`, percent-encoded",
        )
            .into_response();
    };
    match (state.drop_sink)(name, &body) {
        Ok(path) => (StatusCode::OK, path).into_response(),
        Err(err) => (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response(),
    }
}
