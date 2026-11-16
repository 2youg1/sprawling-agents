// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where each tool server one address reaches stands, and what it
//! offers.

use kernel::{Address, AxError, Payload, ServerLabel};
use serde::{Deserialize, Serialize};

/// One line per server the address's configuration names, in the order
/// the configuration names them.
///
/// Order is the file's rather than best-first: somebody scanning this
/// list twice finds the same server in the same place, and a list that
/// reordered itself as servers came and went could not be scanned at
/// all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpHealthAnswer {
    pub addr: Address,
    pub servers: Vec<McpServerHealth>,
}

/// One server, as the handshake found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServerHealth {
    pub label: ServerLabel,
    /// How this server is reached, in the word a person chose: `stdio`,
    /// `http` or `sse`.
    pub transport: String,
    /// The command line or the url, whichever this transport is.
    pub target: String,
    pub state: McpState,
}

/// The three answers a handshake can come back with.
///
/// Exhaustive rather than a flag beside an optional reason: "it is
/// answering", "it wants an account" and "it failed, and here is why"
/// are three different things for a person to do next, and a shape that
/// left the reason optional would let a failure travel without one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum McpState {
    /// It answered, and here is everything it offers.
    Connected {
        /// The revision the far end agreed to, which is not always the
        /// one that was asked for.
        protocol_version: String,
        /// What the server calls itself.
        server: String,
        tools: Vec<McpToolLine>,
    },
    /// It is up and it understood the request, and it wants an account
    /// this city does not hold yet. Its own state rather than a
    /// failure, because what a person does about it is sign in rather
    /// than check the address.
    Authenticating {
        /// What to do about it, as the refusal stated it.
        recovery: String,
    },
    /// It did not answer, and the whole refusal travels: the action that
    /// failed, the subject it failed on, the stable code, and the
    /// recovery a person can act on.
    Failed { refusal: Box<AxError> },
}

/// One tool a server offers, under the name the model sees it by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpToolLine {
    /// The name the server knows it by, which is what a person reading
    /// that server's own documentation will find.
    pub remote: String,
    /// The name this city knows it by: the server's label, then the
    /// remote name. This is what appears in a run's history.
    pub name: String,
    pub disclosure: String,
    /// The tool's input schema, exactly as the server published it. Not
    /// summarised here: a schema is what a person checks a tool's
    /// arguments against, and a summary would be a second, lossy
    /// authority on what the server accepts.
    pub input_schema: Payload,
}
