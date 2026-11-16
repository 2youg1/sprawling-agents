// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod acp;
mod harness;
mod mcp;

pub use acp::{Admitted, Incoming, Progress, admit};
pub use harness::{AcpSession, Answer, HarnessProcess, Listener};
pub use harness::{AgentEntry, AgentId, AgentSource, Consented, Launch, Pin};
pub use harness::{AuthMethod, Introduced, LoginKind, StopReason, Update};
pub use harness::{CLIENT_CONFIGS, configured, detected, passed, pasted, registry_entry};
pub use harness::{Catalog, OFFICIAL, Official, Roster, SetUpDir, Unseated};
pub use harness::{PermissionAsk, Permit, PermitKind, PermitOption};
pub use mcp::{Broker, Connection, Toolkit};
pub use mcp::{EFFECT_META_KEY, Rpc, ScriptedOutbound, tools_from};
pub use mcp::{EXTERNAL_CALL_PATIENCE, Handshake, Listed, McpLink, McpTool, Outbound};
pub use mcp::{Lines, MESSAGE_CEILING, Received, read_one_message};
pub use mcp::{PROTOCOL_VERSION, digits_for_floats, handshake};
#[cfg(feature = "conformance")]
pub use mcp::{counting_starts, echoing, gated};
