// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The port a city takes before it opens, and serving on it
//! (wire-SPEC.md 8-46).
//!
//! Two steps, because the assembly layer needs a gap between them: a
//! city takes its port first, then opens its one writer, and only then
//! has the sinks a [`ServeConfig`] is made of. A single call that bound
//! and served together left the writer open, and writing, before the
//! operating system had said whether the port was free.

use std::net::SocketAddr;

use kernel::{AxCode, AxError, B3Hash};

use crate::reception::{BindFace, BindVerdict, decide_bind};

use super::config::{ServeConfig, router};

/// A listener already bound, and the face its judgement gave it.
///
/// The two travel together because the face is the verdict about this
/// very address: a face carried apart from its listener could be served
/// on a socket it was never judged for.
#[must_use = "a bound port answers nobody until it is served"]
pub struct Bound {
    listener: tokio::net::TcpListener,
    face: BindFace,
}

/// Judges the bind face, then binds the listener.
///
/// # Errors
/// `E_CONFIG_INVALID` from [`decide_bind`], without touching the
/// network, when an exposed address has no pairing token; the same code
/// when the operating system refuses the address, most often because
/// another process holds the port.
pub async fn bind(addr: SocketAddr, token_digest: Option<B3Hash>) -> Result<Bound, AxError> {
    // The face that comes back is the whole of what this listener
    // presents and what it demands; it goes into the shell, where every
    // door reads it rather than reading the configuration again.
    let face = match decide_bind(&addr, token_digest) {
        BindVerdict::Serve(face) => face,
        BindVerdict::Refuse(err) => return Err(err),
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|source| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "bind the control surface",
                format!("{addr}: {source}"),
            )
            .with_recovery("choose a free port, or stop the process already holding it")
        })?;
    Ok(Bound { listener, face })
}

/// Serves on a bound listener until the future is dropped.
///
/// The peer address is read on both faces, because enrolment is refused
/// off this machine even when the listener never left it.
///
/// # Errors
/// Propagates the accept failures the operating system reports.
pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError> {
    let Bound { listener, face } = bound;
    let app = router(&config, face).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await.map_err(|source| {
        AxError::failure(
            AxCode::StorageFatal,
            "serve the control surface",
            source.to_string(),
        )
        .with_recovery("restart the process; the listener is gone")
    })
}
