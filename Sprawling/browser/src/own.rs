// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};

use kernel::{AxCode, AxError};
use serde_json::Value;

use crate::ContextId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnListeners {
    sockets: BTreeSet<SocketAddr>,
    origins: BTreeSet<(String, u16)>,
}

impl OwnListeners {
    #[must_use]
    pub fn new(listeners: impl IntoIterator<Item = SocketAddr>) -> OwnListeners {
        OwnListeners {
            sockets: listeners.into_iter().collect(),
            origins: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_origins(mut self, origins: impl IntoIterator<Item = String>) -> OwnListeners {
        self.origins
            .extend(origins.into_iter().filter_map(|origin| {
                host_and_port(&origin).map(|(host, port)| (host.to_ascii_lowercase(), port))
            }));
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sockets.is_empty() && self.origins.is_empty()
    }

    #[must_use]
    pub fn holds(&self, url: &str) -> bool {
        let Some((host, port)) = host_and_port(url) else {
            return false;
        };
        let name = host.to_ascii_lowercase();
        let literal = host.parse::<IpAddr>().ok().map(|ip| ip.to_canonical());
        let loopback = literal.map_or_else(
            || name == "localhost" || name.ends_with(".localhost"),
            |ip| ip.is_loopback(),
        );
        self.origins.contains(&(name, port))
            || self
                .sockets
                .iter()
                .filter(|at| at.port() == port)
                .any(|at| {
                    let bound = at.ip().to_canonical();
                    loopback
                        || literal == Some(bound)
                        || (bound.is_unspecified() && literal.is_some())
                })
    }

    pub fn refuse(&self, action: &str, url: &str) -> Result<(), AxError> {
        if !self.holds(url) {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{url} is this city's own page"),
        )
        .with_recovery(
            "the city's own pages are the User's; open the address the work serves instead",
        ))
    }
}

#[must_use]
pub fn page_address(tree: &Value, context: &ContextId) -> Option<String> {
    tree.get("contexts")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("context").and_then(Value::as_str) == Some(context.as_str()))?
        .get("url")?
        .as_str()
        .map(str::to_owned)
}

fn host_and_port(url: &str) -> Option<(&str, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let fallback: u16 = match scheme.to_ascii_lowercase().as_str() {
        "http" | "ws" => 80,
        "https" | "wss" => 443,
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => {
            let (inside, after) = bracketed.split_once(']')?;
            (inside, after.strip_prefix(':'))
        }
        None => match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    let port = match port {
        Some(written) => written.parse().ok()?,
        None => fallback,
    };
    (!host.is_empty()).then_some((host, port))
}
