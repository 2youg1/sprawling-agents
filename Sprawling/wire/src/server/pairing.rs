// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[derive(Debug, Deserialize)]
pub struct PairBody {
    #[serde(flatten)]
    pub proof: PairProof,
    pub public_key: String,
    pub label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairProof {
    Open(String),

    Code(String),
}

#[derive(Debug, Serialize)]
pub struct PairAnswer {
    pub device: DeviceId,
}

#[derive(Debug, Serialize)]
pub struct ChallengeAnswer {
    pub nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionBody {
    pub device: DeviceId,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum SessionRequest {
    Known(SessionBody),
    Recover(RecoverySessionBody),
}

#[derive(Debug, Deserialize)]
pub(crate) struct RecoverySessionBody {
    pub public_key: String,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct SessionAnswer {
    pub token: String,
}

#[derive(Debug, Serialize)]
struct RecoveredSessionAnswer {
    token: String,
    device: DeviceId,
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
    Json(body): Json<SessionRequest>,
) -> Response {
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
    let result = match body {
        SessionRequest::Known(body) => state
            .door
            .open_session(&body, origin)
            .map(|token| (StatusCode::OK, Json(SessionAnswer { token })).into_response()),
        SessionRequest::Recover(body) => {
            state
                .door
                .recover_session(body, origin)
                .map(|(device, token)| {
                    (
                        StatusCode::OK,
                        Json(RecoveredSessionAnswer { token, device }),
                    )
                        .into_response()
                })
        }
    };
    result.unwrap_or_else(|err| refusal(&err))
}

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
