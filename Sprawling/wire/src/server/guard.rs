// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::header::{HOST, ORIGIN};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use kernel::{AxCode, AxError};

use crate::reception::{Arrival, Entry, ListenerOrigins, PageHeaders, Presented, decide_entry};

use super::config::refusal_text;

const SEC_FETCH_SITE: &str = "sec-fetch-site";

pub(crate) type Entrance = (Arrival, Arc<ListenerOrigins>);

pub fn entered<S>(routes: Router<S>, arrival: Arrival, origins: &ListenerOrigins) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let names = Arc::new(origins.clone());
    routes
        .route_layer(from_fn_with_state((arrival, Arc::clone(&names)), enter))
        .layer(from_fn_with_state(names, preflight))
}

pub(crate) async fn enter(
    State((arrival, origins)): State<Entrance>,
    request: Request,
    next: Next,
) -> Response {
    let arrival = if request.method() == Method::OPTIONS {
        Arrival::Preflight
    } else {
        arrival
    };
    match judged(&origins, &request, arrival) {
        Some(refused) => refused,
        None => next.run(request).await,
    }
}

pub(crate) async fn preflight(
    State(origins): State<Arc<ListenerOrigins>>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() != Method::OPTIONS {
        return next.run(request).await;
    }
    match judged(&origins, &request, Arrival::Preflight) {
        Some(refused) => refused,
        None => next.run(request).await,
    }
}

fn judged(origins: &ListenerOrigins, request: &Request, arrival: Arrival) -> Option<Response> {
    let headers = request.headers();
    let read = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let presented = Presented {
        host: read(HOST.as_str()),
        origin: read(ORIGIN.as_str()),
        sec_fetch_site: read(SEC_FETCH_SITE),
    };
    match decide_entry(origins, presented, arrival) {
        Entry::Refused(why) => {
            Some((StatusCode::FORBIDDEN, refusal_text(&why.error())).into_response())
        }
        Entry::Page | Entry::Caller(_) => None,
    }
}

#[derive(Debug, Clone)]
pub struct Stamp(Vec<(HeaderName, HeaderValue)>);

impl Stamp {
    pub(crate) fn of(headers: &PageHeaders, stored: Stored) -> Result<Self, AxError> {
        let extra = match stored {
            Stored::AsPage => None,
            Stored::Never => Some(("cache-control", "no-store".to_owned())),
        };
        headers
            .lines()
            .iter()
            .cloned()
            .chain(extra)
            .map(|(name, value)| {
                let name = HeaderName::try_from(name).map_err(|_| unspellable(name))?;
                let value = HeaderValue::try_from(value).map_err(|_| unspellable(name.as_str()))?;
                Ok((name, value))
            })
            .collect::<Result<Vec<_>, AxError>>()
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stored {
    AsPage,
    Never,
}

pub(crate) async fn stamp(
    State(stamp): State<Arc<Stamp>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in &stamp.0 {
        headers.insert(name.clone(), value.clone());
    }
    response
}

fn unspellable(name: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "spell the listener's response headers",
        format!("`{name}` is not a valid HTTP header"),
    )
    .with_recovery("report this: the headers are this build's own text")
}
