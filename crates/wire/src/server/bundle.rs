// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Serving the client bundle over HTTP: headers on, policy out. Which
//! bytes answer a path is `wire::assets`' decision.

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

/// The two routes that answer the client bundle: `/` and every path
/// below it that no other route of the same table claims.
///
/// The city's own port and the remote listener (`bin::outside`) serve
/// the page through this one table, so a device that opens the remote
/// address gets the same bytes as a browser on this machine
/// (`crates/wire/Spec.lean` §8-2); `headers` are the listener's own,
/// generated once per listener (`crates/wire/spec/Reception/Entry.lean`
/// §8-94). The routes take no credential: a browser has to load the page
/// before it has anywhere to pair.
///
/// # Errors
/// `headers` cannot be spelled as HTTP headers.
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
