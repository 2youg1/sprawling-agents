// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three posted bodies: an outside editor's request, a recording,
//! and a file dropped onto the composer. Each is read just far enough to
//! hand inward, and the city's answer comes back as the response to the
//! same request.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::reception::{Admission, Door, decide_admission};

use super::config::ShellState;
use super::reply::refusal_text;

/// An outside editor's request. The token is judged here and the
/// verdict travels inward; an unauthenticated request still reaches the
/// admission, because what it may learn is that admission's to decide.
///
/// The body is read as JSON and passed on unread. Which keys make a
/// request, and what each of them must hold, is `protocol::Incoming`'s
/// single answer; a second reading here would be a second grammar, and
/// the two would first disagree about an empty field.
pub(crate) async fn accept_acp(State(state): State<Arc<ShellState>>, body: Bytes) -> Response {
    let Ok(request) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send {\"token\":..,\"addr\":..,\"task\":..,\"goal\":..}",
        )
            .into_response();
    };
    // The token is judged by `decide_admission`, which is the same
    // judgement the acting doors are layered with; this door differs
    // only in where the token is written (a body key, because that is
    // what an editor sends) and in admitting an unpaired caller so the
    // admission can word what it may learn.
    let pairing = match decide_admission(
        Door::Acp,
        request.get("token").and_then(serde_json::Value::as_str),
        &state.face,
    ) {
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

/// One recording in, one line of text back.
///
/// The media type is read off the request rather than guessed from the
/// bytes: a browser records into whatever container it has, and it is
/// the only party that knows which. A request with none is refused by
/// name, because a container nobody declared cannot be sent on.
pub(crate) async fn accept_recording(
    State(state): State<Arc<ShellState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // The parameters a browser appends (`; codecs=opus`) name the codec
    // inside the container, and what the audio wire routes on is the
    // container, so a value of parameters alone declares none.
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

/// One dropped file in, the absolute path the city kept it at back.
///
/// The name rides the query string rather than a header, because a
/// header value carries ASCII reliably and a person's file names are
/// often not ASCII (channels-SPEC.md 8-49).
pub(crate) async fn accept_drop(
    State(_state): State<Arc<ShellState>>,
    Query(_params): Query<HashMap<String, String>>,
    _body: Bytes,
) -> Response {
    StatusCode::NOT_IMPLEMENTED.into_response()
}
