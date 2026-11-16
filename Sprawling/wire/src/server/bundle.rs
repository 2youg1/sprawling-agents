// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use kernel::AxError;

use crate::assets::{AssetReply, ClientAssets};
use crate::reception::PageHeaders;

use super::config::refusal_text;
use super::guard::{Stamp, Stored, stamp};

pub fn bundle_routes<S>(
    client: Arc<ClientAssets>,
    headers: &PageHeaders,
) -> Result<Router<S>, AxError>
where
    S: Clone + Send + Sync + 'static,
{
    let stamped = Arc::new(Stamp::of(headers, Stored::AsPage)?);
    Ok(Router::new()
        .route("/", get(serve_index))
        .route("/{*asset}", get(serve_asset))
        .with_state(client)
        .layer(from_fn_with_state(stamped, stamp)))
}

async fn serve_index(State(client): State<Arc<ClientAssets>>) -> Response {
    asset_response(client.lookup("index.html"))
}

async fn serve_asset(
    State(client): State<Arc<ClientAssets>>,
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> Response {
    asset_response(client.lookup(&asset))
}

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
