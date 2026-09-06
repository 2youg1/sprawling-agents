// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reaching a Model Context Protocol server.
//!
//! The city does not know any particular server. Which one a building
//! talks to is that building's configuration; what this module knows is
//! the protocol, and the protocol is the same whether the far end is a
//! hosted catalogue of a thousand applications or a script somebody
//! wrote this morning.
//!
//! Two things about the current revision shape the code. The list of
//! tools may not vary per connection, which is the same rule as freezing
//! a run's tool table — the two arrived from opposite directions and
//! agree, so the tool table is read once and frozen with the run. And
//! every connection opens with the lifecycle the specification defines:
//! `initialize`, then a `notifications/initialized` notification, before
//! any other request. That is why the seam has two methods rather than
//! one — a notification is a message with no answer, and pretending it
//! has one is how a client ends up waiting for a 202 with no body.
//!
//! Everything a server returns is other people's text. It lands on the
//! same tool seam as the local tools, so it enters the taint ring the
//! same way, and there is no unwrapping face here.

mod handshake;
mod outbound;
mod tools;

pub use handshake::PROTOCOL_VERSION;
pub use handshake::{Handshake, Rpc, handshake};
pub use outbound::{EXTERNAL_CALL_PATIENCE, Outbound, ScriptedOutbound, digits_for_floats};
pub use tools::{Listed, McpTool, tools_from};
