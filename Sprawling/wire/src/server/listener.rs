// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;

use kernel::{AxCode, AxError, B3Hash};

use crate::reception::{BindFace, BindVerdict, ListenerOrigins, decide_bind};

use super::config::{ServeConfig, router};

#[must_use = "a bound port answers nobody until it is served"]
pub struct Bound {
    listener: tokio::net::TcpListener,
    local: SocketAddr,
    face: BindFace,
    origins: ListenerOrigins,
}

impl Bound {

    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    #[must_use]
    pub fn origins(&self) -> &ListenerOrigins {
        &self.origins
    }
}

pub async fn bind(addr: SocketAddr, key: Option<B3Hash>) -> Result<Bound, AxError> {

    let face = match decide_bind(&addr, key) {
        BindVerdict::Serve(face) => face,
        BindVerdict::Refuse(err) => return Err(err),
    };
    let unbound = |source: std::io::Error| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "bind the control surface",
            format!("{addr}: {source}"),
        )
        .with_recovery("choose a free port, or stop the process already holding it")
    };
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(unbound)?;
    let local = listener.local_addr().map_err(unbound)?;
    Ok(Bound {
        listener,
        local,
        face,
        origins: ListenerOrigins::of(local),
    })
}

pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError> {
    let Bound {
        listener,
        face,
        origins,
        ..
    } = bound;
    let app = router(&config, face, &origins)?.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await.map_err(|source| {
        AxError::failure(
            AxCode::StorageFatal,
            "serve the control surface",
            source.to_string(),
        )
        .with_recovery("restart the process; the listener is gone")
    })
}
