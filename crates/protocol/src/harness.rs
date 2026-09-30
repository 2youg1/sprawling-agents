// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The official harnesses this city drives as ACP agents: which five
//! (`roster`), and one session with one of them (`session`)
//! (agent_protocols-SPEC.md 8-19).

mod process;
mod reading;
mod roster;
mod session;

pub use process::HarnessProcess;
pub use reading::Lines;
pub use roster::{Harness, Launch, Program};
pub use session::{
    AcpSession, Answer, Listener, PermissionAsk, Permit, PermitKind, PermitOption, StopReason,
    Update,
};
