// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod broker;
mod handshake;
mod http;
mod link;
mod outbound;
mod reading;
mod redeeming;
mod sse;
mod stdio;
mod tools;

pub use broker::{Broker, Connection, Toolkit};
pub use handshake::PROTOCOL_VERSION;
pub use handshake::{Handshake, Rpc, handshake};
pub use link::McpLink;
pub use outbound::{EXTERNAL_CALL_PATIENCE, Outbound, ScriptedOutbound, digits_for_floats};
pub(crate) use reading::Heard;
pub use reading::{Lines, MESSAGE_CEILING, Received, read_one_message};
#[cfg(feature = "conformance")]
pub use stdio::{counting_starts, echoing, gated};
pub use tools::{EFFECT_META_KEY, Listed, McpTool, tools_from};
