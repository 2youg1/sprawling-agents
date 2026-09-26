// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Serving the client bundle over HTTP: headers on, policy out. Which
//! bytes answer a path is `channels::assets`' decision.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::assets::AssetReply;

use super::config::ShellState;
use super::reply::refusal_text;

pub(crate) async fn serve_index(State(state): State<Arc<ShellState>>) -> Response {
    asset_response(state.client.lookup("index.html"))
}

pub(crate) async fn serve_asset(
    State(state): State<Arc<ShellState>>,
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> Response {
    asset_response(state.client.lookup(&asset))
}

/// The shell around [`ClientAssets::lookup`]: headers on, policy out.
fn asset_response(reply: AssetReply) -> Response {
    match reply {
        AssetReply::Found {
            bytes,
            content_type,
            gzipped: true,
        } => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CONTENT_ENCODING, "gzip"),
            ],
            bytes,
        )
            .into_response(),
        AssetReply::Found {
            bytes,
            content_type,
            gzipped: false,
        } => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            bytes,
        )
            .into_response(),
        AssetReply::Miss(err) => (StatusCode::NOT_FOUND, refusal_text(&err)).into_response(),
    }
}
