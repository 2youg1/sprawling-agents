// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a call to a provider stops, stage by stage.
//!
//! **A refusal reading `error sending request for url (...): operation
//! timed out` tells a person nothing they can act on.** Four things go
//! wrong before a provider ever sees a request, and each has a different
//! next step: the name does not resolve (check the spelling, or the
//! proxy), the connection is refused or never answers (a firewall, a
//! wrong port, a proxy that is not running), the handshake fails (a
//! hostname that is not a legal DNS name, a certificate this machine
//! does not trust), or the provider answers with a status (401 is a key,
//! 404 is a path). This is the vocabulary of that reading; `gateway`
//! takes it, because the reading needs a socket and this crate has none.

use serde::{Deserialize, Serialize};

/// How the name resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Named {
    /// How many addresses came back. A name that resolves to nothing is
    /// `NotFound`, never `Resolved(0)`.
    Resolved(u32),
    /// The resolver knows the name does not exist.
    NotFound,
    /// The resolver itself failed, and said this.
    Refused(String),
    /// A proxy resolves the name, so this machine did not.
    ProxiedAway,
}

/// Whether a socket opened to the first address the name gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Connected {
    Open,
    /// Something answered and said no: a closed port, or a firewall
    /// that refuses rather than drops.
    Refused,
    /// Nothing answered inside the deadline. The most common shape of a
    /// blocked network, and the one that looks like a slow provider.
    Silent,
    Failed(String),
    /// Not attempted: the name did not resolve, or a proxy connects.
    Skipped,
}

/// What the request itself did, once the transport was somebody else's
/// problem. Carries the handshake, because a TLS failure surfaces as a
/// request that never got a status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Answered {
    /// The provider answered. The status is the whole finding: 200 is
    /// reachable, 401 is a key, 404 is a base URL with a path on it.
    Status(u16),
    /// The name is not one TLS can put in a handshake — an underscore,
    /// a trailing dot, a label that is not ASCII.
    NameNotUsable(String),
    /// The handshake failed for any other reason, certificates
    /// included.
    HandshakeFailed(String),
    /// The request never reached a handshake.
    Unreachable(String),
}

/// What the city believes it will send this request through.
///
/// Four of the five readings mean "no proxy", and they are kept apart
/// because the next step differs for each: nothing names one, the
/// person's own `NO_PROXY` excludes this host, the city's rule takes an
/// address on this machine off the proxy, or the person settled
/// [`Proxying::Never`] for this endpoint. A person whose call fails can
/// act on the reason and cannot act on "direct".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Through {
    /// Straight out, as far as this machine's environment says.
    Direct,
    /// An environment variable named it, and this is the variable.
    Environment(String),
    /// This host is on `NO_PROXY`, so the variables do not apply to it.
    Excluded,
    /// The address is on the machine the city runs on, and
    /// [`Proxying::ExceptLocal`] keeps such a call off the proxy.
    LocalAddress,
    /// The person settled [`Proxying::Never`] for this endpoint, so no
    /// proxy applies whatever this machine is configured with.
    Disabled,
}

/// Which of this endpoint's calls go through the machine's proxy.
///
/// A proxy that intercepts loopback answers 502 for a local inference
/// server, so the city takes an address on its own machine off the
/// proxy by default — and that default is a rule about the common
/// machine, not a fact about every machine. Two other settings are real
/// needs rather than symmetry: a person whose provider sits behind a
/// relay on loopback that their organisation requires them to audit
/// needs [`Always`](Proxying::Always), and a person on a virtual network
/// adapter, whose routes already carry every packet, needs
/// [`Never`](Proxying::Never) so a stale proxy variable cannot break a
/// call the machine would otherwise make.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Proxying {
    /// The machine's proxy applies, except to an address on this
    /// machine. What a person who has not thought about proxies wants.
    #[default]
    ExceptLocal,
    /// The machine's proxy applies to every call, loopback included.
    Always,
    /// No proxy applies, whatever this machine is configured with.
    Never,
}

/// One staged reading of one host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Reach {
    pub host: String,
    pub named: Named,
    pub connected: Connected,
    pub answered: Answered,
    pub through: Through,
    /// Stamped by the caller, which is the one place a clock is read.
    pub elapsed_ms: u64,
}

impl Reach {
    /// Whether the provider answered at all, whatever it answered.
    pub fn answered_at_all(&self) -> bool {
        matches!(self.answered, Answered::Status(_))
    }
}
