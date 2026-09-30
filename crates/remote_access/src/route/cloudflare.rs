// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The default route: a Cloudflare named tunnel (remote_access-SPEC.md §8-8).

use std::net::SocketAddr;
use std::path::PathBuf;

use kernel::{AxCode, AxError, TimeoutMs};

use super::{Opened, PublicUrl, Route};

/// A locally managed named tunnel, as the person made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tunnel {
    pub program: PathBuf,
    pub name: TunnelName,
    pub url: PublicUrl,
}

/// A tunnel's name as `cloudflared` takes it on its command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelName(String);

/// The Cloudflare named tunnel route.
pub struct NamedTunnel {
    tunnel: Tunnel,
    patience: TimeoutMs,
}

impl TunnelName {
    /// # Errors
    /// Not built yet.
    pub fn parse(text: &str) -> Result<Self, AxError> {
        Err(not_built(text))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl NamedTunnel {
    #[must_use]
    pub fn new(tunnel: Tunnel, patience: TimeoutMs) -> Self {
        Self { tunnel, patience }
    }
}

impl Route for NamedTunnel {
    fn open(&mut self, _local: SocketAddr) -> Result<Opened, AxError> {
        Err(not_built(&format!(
            "{:?} {}",
            self.tunnel.program, self.patience.0
        )))
    }

    fn close(&mut self) -> Result<(), AxError> {
        Err(not_built("close"))
    }
}

fn not_built(subject: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "open a route", subject)
        .with_recovery("the route is not built yet")
}
