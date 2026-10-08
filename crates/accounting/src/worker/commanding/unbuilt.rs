// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The verbs the wire spells before this city can perform them, and the
//! refusal each one is answered with.

use kernel::AxError;

use super::super::not_built;

/// A verb the wire spells and this city cannot perform, with what the
/// refusal names. Each is answered by its own arm of `run_command`, so a
/// command added without an executor stops the build there.
pub(super) enum Unbuilt<'a> {
    HandOff(&'a kernel::ApprovalId),
    PutShelved(String),
    BatchByBuilding(&'a kernel::Address),
    Auth,
    CloseCity,
    AddAgent,
    AgentLogin(String),
    ForgetDevice(&'a wire::DeviceId),
}

impl Unbuilt<'_> {
    /// The `not_built` refusal the city owes a peer that asks for this
    /// verb anyway; each arm of `run_command` names it, which is how the
    /// wiring gate reads that the verb has no executor.
    pub(super) fn not_built(self) -> AxError {
        match self {
            Unbuilt::HandOff(item) => not_built(
                "hand a question to somebody else",
                item.as_str().to_owned(),
                "answer it yourself, or appoint that resident as the delegate; handing one \
                 question on is not built",
            ),
            Unbuilt::PutShelved(name) => not_built(
                "write a shelved document",
                name,
                "edit the file under the shelf by hand; writing it from the page is not built",
            ),
            Unbuilt::BatchByBuilding(addr) => not_built(
                "run a building's work as one batch",
                addr.as_str().to_owned(),
                "dispatch the rooms one at a time; batching a building is not built",
            ),
            Unbuilt::Auth => not_built(
                "authenticate over the command channel",
                "Auth".to_owned(),
                "the pairing token is proved in the handshake, not in a command",
            ),
            Unbuilt::CloseCity => not_built(
                "close the city",
                "CloseCity".to_owned(),
                "close it from the terminal it runs in; closing it from a page is not built",
            ),
            Unbuilt::AddAgent => not_built(
                "add an ACP agent",
                "AddAgent".to_owned(),
                "write the agent's row in the city's CONFIG.toml; adding it from a page is not                  built",
            ),
            Unbuilt::AgentLogin(agent) => not_built(
                "start an agent's login",
                agent,
                "sign in inside the agent itself; starting its login from the city is not built",
            ),
            Unbuilt::ForgetDevice(device) => not_built(
                "forget a paired browser",
                device.as_str().to_owned(),
                "revoking a paired browser is not built",
            ),
        }
    }
}
