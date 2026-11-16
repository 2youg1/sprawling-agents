// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The port a city takes before it opens, and serving on it
//! (`crates/wire/spec/Server/Listener.lean` §8-46).
//!
//! Two steps, because the assembly layer needs a gap between them: a
//! city takes its port first, then opens its one writer, and only then
//! has the sinks a [`ServeConfig`] is made of. A single call that bound
//! and served together left the writer open, and writing, before the
//! operating system had said whether the port was free.

use std::net::SocketAddr;

use kernel::{AxCode, AxError, B3Hash};

use crate::reception::{BindFace, BindVerdict, ListenerOrigins, decide_bind};

use super::config::{ServeConfig, router};

/// A listener already bound, the address it holds, and the face its
/// judgement gave it.
///
/// The face travels with the listener because it is the verdict about
/// this very address: a face carried apart from its listener could be
/// served on a socket it was never judged for. The address is read once,
/// at bind, because a city asked to serve on `:0` holds a port only the
/// listener knows (wire D16).
#[must_use = "a bound port answers nobody until it is served"]
pub struct Bound {
    listener: tokio::net::TcpListener,
    local: SocketAddr,
    face: BindFace,
    origins: ListenerOrigins,
}

impl Bound {
    /// The address the listener holds: the port the operating system
    /// gave when `bind` was asked for port 0, the one asked for
    /// otherwise. Every reader that hands the city's address on reads
    /// this one, never the address `bind` was given.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// The names and origins this listener answers to, computed once from
    /// the address it holds (`crates/wire/spec/Reception/Entry.lean`
    /// §8-94): the URL a person opens, and what the residents' browser
    /// guard keeps them away from.
    #[must_use]
    pub fn origins(&self) -> &ListenerOrigins {
        &self.origins
    }
}

/// Judges the bind face, then binds the listener.
///
/// # Errors
/// `E_CONFIG_INVALID` from [`decide_bind`], without touching the
/// network, when no key is given; the same code
/// when the operating system refuses the address, most often because
/// another process holds the port, or cannot say which address it gave.
pub async fn bind(addr: SocketAddr, key: Option<B3Hash>) -> Result<Bound, AxError> {
    // The face that comes back is the whole of what this listener
    // presents and what it demands; it goes into the shell, where every
    // door reads it rather than reading the configuration again.
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

/// Serves on a bound listener until the future is dropped.
///
/// The peer address is read on both faces, because enrolment is refused
/// off this machine even when the listener never left it.
///
/// # Errors
/// Propagates the accept failures the operating system reports.
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
