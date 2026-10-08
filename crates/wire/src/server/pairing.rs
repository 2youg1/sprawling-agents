// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's door for a browser: pairing, and the challenge and
//! signature that open a session (`crates/wire/spec/Server.lean` §8-93).
//!
//! The bodies are read in their final shape; every route answers that the
//! door is not built yet, until the pairing, the challenge and the entry
//! decision land behind them.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

use crate::answer::DeviceId;

use super::config::refusal_text;

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

pub(super) async fn accept_pairing(Json(body): Json<PairBody>) -> Response {
    let PairBody { proof, .. } = body;
    let action = match proof {
        PairProof::Open(_) => "pair a browser with an open code",
        PairProof::Code(_) => "pair a browser with a pairing code",
    };
    not_built(action, "/pair")
}

pub(super) async fn offer_challenge() -> Response {
    not_built("offer a session challenge", "/session/challenge")
}

pub(super) async fn open_session(Json(body): Json<SessionBody>) -> Response {
    not_built("open a browser session", body.device.as_str())
}

/// The answer a route owes until the local door is built behind it.
fn not_built(action: &'static str, subject: &str) -> Response {
    let refused = AxError::failure(AxCode::ToolUnavailable, action, subject.to_owned())
        .with_recovery("this build does not pair browsers at its own door yet");
    (StatusCode::NOT_IMPLEMENTED, refusal_text(&refused)).into_response()
}
