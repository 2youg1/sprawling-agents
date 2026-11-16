// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's door for a browser: pairing, and the challenge and
//! signature that open a session (`crates/wire/spec/Server.lean` §8-93).
//!
//! The entry decision has already let only a browser of this listener's
//! own origin through (`crates/wire/spec/Reception/Entry.lean` §8-94);
//! the rules of what each route then decides are the door's
//! (`server::door`, `reception::pairing`).

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::header::ORIGIN;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

use crate::answer::DeviceId;

use super::config::{ShellState, refusal_text};

/// `POST /pair`: one of the two codes, the browser's public device key,
/// and the name the page gives itself.
#[derive(Debug, Deserialize)]
pub struct PairBody {
    #[serde(flatten)]
    pub proof: PairProof,
    pub public_key: String,
    pub label: String,
}

/// Which code a pairing presents: exactly one of the two.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairProof {
    /// The open code `/web` handed over through the user-only redirect file.
    Open(String),
    /// The pairing code the terminal shows.
    Code(String),
}

/// What a pairing answers: the id the city gave the browser.
#[derive(Debug, Serialize)]
pub struct PairAnswer {
    pub device: DeviceId,
}

/// What `POST /session/challenge` answers: the nonce the device signs.
#[derive(Debug, Serialize)]
pub struct ChallengeAnswer {
    pub nonce: String,
}

/// `POST /session`: the device, the nonce it was given, and its signature
/// over that nonce.
#[derive(Debug, Deserialize)]
pub struct SessionBody {
    pub device: DeviceId,
    pub nonce: String,
    pub signature: String,
}

/// What a session answers: the token the page keeps in memory and shows
/// on every later request.
#[derive(Debug, Serialize)]
pub struct SessionAnswer {
    pub token: String,
}

pub(super) async fn accept_pairing(
    State(state): State<Arc<ShellState>>,
    Json(body): Json<PairBody>,
) -> Response {
    match state.door.pair(&body.proof, &body.public_key, &body.label) {
        Ok(device) => (StatusCode::OK, Json(PairAnswer { device })).into_response(),
        Err(err) => refusal(&err),
    }
}

pub(super) async fn offer_challenge(State(state): State<Arc<ShellState>>) -> Response {
    match state.door.challenge() {
        Ok(nonce) => (StatusCode::OK, Json(ChallengeAnswer { nonce })).into_response(),
        Err(err) => refusal(&err),
    }
}

pub(super) async fn open_session(
    State(state): State<Arc<ShellState>>,
    headers: HeaderMap,
    Json(body): Json<SessionBody>,
) -> Response {
    // The entry decision admitted only a browser of this listener's own
    // origin, and a browser is a request that carried one; the signature
    // is checked over the origin it was made on.
    let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) else {
        return refusal(
            &AxError::failure(
                AxCode::GateDenied,
                "open a browser session",
                "the request carried no Origin",
            )
            .with_recovery("open a session from the city's own page"),
        );
    };
    match state.door.open_session(&body, origin) {
        Ok(token) => (StatusCode::OK, Json(SessionAnswer { token })).into_response(),
        Err(err) => refusal(&err),
    }
}

/// The status a refusal of the door is answered with.
fn refusal(err: &AxError) -> Response {
    let code = err.code();
    let status = if matches!(code, AxCode::PairingRefused | AxCode::GateDenied) {
        StatusCode::FORBIDDEN
    } else if matches!(code, AxCode::InvalidArgs) {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, refusal_text(err)).into_response()
}
