// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::{AxCode, AxError, GateRefusal};

use super::{EgressTarget, GateOutcome};

#[must_use]
pub fn attach(endpoint: Option<&EgressTarget>) -> GateOutcome {
    match endpoint {
        None => GateOutcome::Ask {
            question: Box::new(
                AxError::refusal(
                    AxCode::ApprovalPending,
                    "attach to the User's browser",
                    "no browser address is declared",
                    GateRefusal::new(
                        "a run drives the User's browser only where the User has told \
                         it to listen",
                        "this building enabled `usersbrowser` and declared no address",
                        "ask the User for the address their browser is listening on, and \
                         have them put it on a `usersbrowser` key in RULES.toml",
                    ),
                )
                .with_recovery(
                    "start the browser with remote debugging, copy the ws://127.0.0.1:<port>/session \
                     address it prints, and add `usersbrowser = \"<that address>\"` to the building's \
                     RULES.toml; the next dispatch picks it up. Only the User can do this, \
                     because it is their logins the attachment reaches",
                ),
            ),
        },
        Some(EgressTarget::Loopback) => GateOutcome::Allow,
        Some(other) => GateOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::GateDenied,
                    "attach to the User's browser",
                    subject_of(other),
                    GateRefusal::new(
                        "a browser attachment stays on this machine",
                        "the declared address is not loopback",
                        "declare the ws://127.0.0.1:<port>/session address the browser \
                         itself prints",
                    ),
                )
                .with_recovery(
                    "attach only reaches a browser on this machine; declare a loopback \
                     address, or keep using the `browser` tool, which this city starts \
                     itself",
                ),
            ),
        },
    }
}

fn subject_of(target: &EgressTarget) -> String {
    match target {
        EgressTarget::Loopback => "loopback".to_owned(),
        EgressTarget::Private => "a host on the same private network".to_owned(),
        EgressTarget::Public { host } => host.clone(),
        EgressTarget::Connector { label } => format!("connector {}", label.as_str()),
    }
}
