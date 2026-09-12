// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The broker that holds an outside application's OAuth.
//!
//! `protocol::mcp` connects to any tool server and has never heard of
//! any particular one. This is the single module allowed to know that a
//! given server is Composio's, and what it knows is confined to one
//! job: turning "connect my GitHub" into a consent page the person can
//! open, and reading back which applications are connected.
//!
//! It lives in `gateway` because that is where reaching outside this
//! machine is already decided - one proxy policy, one credential
//! custody - and nowhere else has both. It returns its own vocabulary
//! rather than wire shapes: the assembly layer maps them, exactly as it
//! already maps a `protocol::mcp` handshake into `McpState`.
//!
//! One broker exists, so there is no trait here. A second outsourced
//! service is what would earn one; a trait with a single implementation
//! is a seam nobody has found yet.

mod broker;

pub use broker::{Broker, Connection, Toolkit};
