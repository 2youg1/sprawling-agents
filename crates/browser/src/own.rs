// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether an address is one of the city's own listeners, and which
//! address a tab is on (`crates/browser/Spec.lean` D13).
//!
//! A resident's browser that reached the city's own page would act there
//! with whatever the profile holds for it; in the person's own browser
//! that is the person's device key. The decision is pure: the listeners
//! are handed in, and a name is judged by its spelling and never
//! resolved, because a lookup would make the answer depend on the
//! network.

use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};

use kernel::{AxCode, AxError};
use serde_json::Value;

use crate::ContextId;

/// The addresses the city listens on at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnListeners(BTreeSet<SocketAddr>);

impl OwnListeners {
    #[must_use]
    pub fn new(listeners: impl IntoIterator<Item = SocketAddr>) -> OwnListeners {
        OwnListeners(listeners.into_iter().collect())
    }

    /// Whether no listener is registered, so no address can be held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `url` reaches one of these listeners: an `http`, `https`,
    /// `ws` or `wss` address on a listener's port (the scheme's default
    /// when none is written) whose host is this machine's loopback, the
    /// address the listener is bound to, or, for a listener bound to the
    /// unspecified address, any address literal.
    #[must_use]
    pub fn holds(&self, url: &str) -> bool {
        let Some((host, port)) = host_and_port(url) else {
            return false;
        };
        let literal = host.parse::<IpAddr>().ok().map(|ip| ip.to_canonical());
        let loopback = literal.map_or_else(
            || {
                let name = host.to_ascii_lowercase();
                name == "localhost" || name.ends_with(".localhost")
            },
            |ip| ip.is_loopback(),
        );
        self.0.iter().filter(|at| at.port() == port).any(|at| {
            let bound = at.ip().to_canonical();
            loopback || literal == Some(bound) || (bound.is_unspecified() && literal.is_some())
        })
    }

    /// Refuses `url` when it reaches one of these listeners.
    ///
    /// # Errors
    /// `E_GATE_DENIED`, naming the action and the address.
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

/// The address `context` is on, as a `browsingContext.getTree` reply
/// states it; `None` when the reply does not list that tab or gives it no
/// address.
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

/// The host and port of a URL a browser could reach a listener at.
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
