// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ACP agents this city drives: what an entry is and how a person
//! consents to it (`entry`), the registry catalog (`catalog`), the five
//! official harnesses as built-in entries (`roster`), a pasted agent
//! (`paste`), the environment a child sees (`environment`), and one
//! session with an agent (`process`, `session`)
//! (`crates/agent_protocols/Spec.lean` §8-19).

mod catalog;
mod entry;
mod environment;
mod paste;
mod process;
mod roster;
mod session;

pub use catalog::{Catalog, registry_entry};
pub use entry::{AgentEntry, AgentId, AgentSource, Consented, Launch, Pin};
pub use environment::passed;
pub use paste::pasted;
pub use process::HarnessProcess;
pub use roster::{OFFICIAL, Official, Roster, SetUpDir, Unseated};
pub use session::{
    AcpSession, Answer, AuthMethod, Introduced, Listener, LoginKind, PermissionAsk, Permit,
    PermitKind, PermitOption, StopReason, Update,
};
