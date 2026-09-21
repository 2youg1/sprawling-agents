// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one door that asks a person.
//!
//! Every other door answers from the rules it was handed. This one
//! cannot: what it would grant is not an action but the whole of the
//! login state inside a browser the person is already using - mail,
//! banking, the company console - and the per-building isolation that
//! `browser::Profile` keeps is exactly what stops applying the moment a
//! run attaches to the browser a person drives. The person's own
//! action is the only answer the platform offers: they turn remote
//! debugging on, in the running browser, themselves.
//!
//! So the verdict has three arms here and exactly one door in the
//! roster returns the third. The recovery sentence is the question: it
//! names the step only the person can take.

use crate::error::{AxCode, AxError, GateRefusal};

use super::{EgressTarget, GateOutcome};

/// May a run attach to the person's browser?
///
/// `endpoint` is the address the person declared for their browser, as
/// the egress classifier read it. `None` is a building that enabled the
/// tool and has not said where the browser answers, which is the state
/// the person's next action resolves.
///
/// # The rule
/// A declared loopback address is the person's own machine reached from
/// the person's own machine: attach. No declaration is a question for
/// the person, not a refusal, because they can answer it. Any other
/// address would read that login state over a network, and the answer
/// there is no.
#[must_use]
pub fn attach(endpoint: Option<&EgressTarget>) -> GateOutcome {
    match endpoint {
        None => GateOutcome::Ask {
            question: Box::new(
                AxError::refusal(
                    AxCode::ApprovalPending,
                    "attach to the person's browser",
                    "no browser address is declared",
                    GateRefusal::new(
                        "a run drives the person's browser only where the person has told \
                         it to listen",
                        "this building enabled `usersbrowser` and declared no address",
                        "ask the person for the address their browser is listening on, and \
                         have them put it on a `usersbrowser` key in RULES.toml",
                    ),
                )
                .with_recovery(
                    "start the browser with remote debugging, copy the ws://127.0.0.1:<port>/session \
                     address it prints, and add `usersbrowser: <that address>` to the building's \
                     RULES.toml; the next dispatch picks it up. Only the person can do this, \
                     because it is their logins the attachment reaches",
                ),
            ),
        },
        Some(EgressTarget::Loopback) => GateOutcome::Allow,
        Some(other) => GateOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::GateDenied,
                    "attach to the person's browser",
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

/// What to name as the subject of the refusal: the far end, in the
/// terms the target was classified with. The raw declared URL never
/// appears, because a URL can carry a credential in its query.
fn subject_of(target: &EgressTarget) -> String {
    match target {
        EgressTarget::Loopback => "loopback".to_owned(),
        EgressTarget::Private => "a host on the same private network".to_owned(),
        EgressTarget::Public { host } => host.clone(),
        EgressTarget::Connector { label } => format!("connector {}", label.as_str()),
    }
}
