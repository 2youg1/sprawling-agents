// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The doors: which gate one call's declared Effect names, and what
//! that gate's verdict means to the bench.

use super::*;

impl ToolBench {
    /// The door this call's declared Effect names.
    ///
    /// `None` means the door is open and the tool may run; `Some` means
    /// the door answered for this call and the tool does not run. The
    /// answer is the bench's, not the gate's: a refusal flows back as a
    /// tool result rather than ending the turn, and an asking door's
    /// question reaches the model the same way.
    ///
    /// `subject` is what the tool read off its own arguments (M-17).
    /// The bench used to spell an argument name by hand here, which
    /// judged the browser tool by a `host` key it never wrote; now the
    /// tool's grammar answers and this function only matches.
    ///
    /// # Errors
    /// Refuses a call that declares an effect this bench does not route,
    /// a subject its effect does not read, a spawn or a rule change on a
    /// bench built without a job, and arguments that will not serialise
    /// for the secret scan.
    pub(super) fn admit(
        &mut self,
        call: &ToolCall,
        name: &str,
        effect: &Effect,
        subject: &GateSubject,
    ) -> Result<Option<BenchOutcome>, AxError> {
        match effect {
            Effect::Read => {}
            // The tool declares an area, and a call may name a narrower
            // one. Both are asked the same question, and both must pass:
            // the declaration is what the tool promised to reach, and
            // the call's own area is what it now reaches for.
            Effect::Write { domain: area } => {
                let verdict = kernel::gate::reach(&self.domain, area, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
                let named = match subject {
                    GateSubject::Area(at) | GateSubject::Room(at) => Some(at),
                    GateSubject::Scope(_) | GateSubject::Host(_) | GateSubject::None => None,
                };
                if let Some(at) = named {
                    let verdict = kernel::gate::reach(&self.domain, at, &self.taint);
                    if let Some(answered) = self.settled(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::Connector { label } => {
                // Same door, same scan; only the target differs. A
                // connector's destination is its registration's, so
                // there is nothing for the call to name and nothing for
                // a model to get wrong.
                let spans = kernel::secret::scan(&scanned(call, "scan connector args")?);
                let verdict = kernel::gate::egress(
                    &spans,
                    &EgressTarget::Connector {
                        label: label.clone(),
                    },
                    self.prior_public_egress,
                );
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }
                // A second door, behind the first, answering a
                // different question. The egress door asked whether
                // these bytes may leave; this one asks whether what
                // happens at the other end can be taken back. The
                // desktop connector is the first thing here for which
                // the answer can be no: a key pressed on somebody's own
                // machine has no restoration, so the floor's `[sandbox]`
                // decides, and content that came in from outside is
                // refused whatever the floor says.
                let called = ToolName::parse(name)?;
                let reaching = kernel::ConnectorCall {
                    label,
                    tool: &called,
                };
                if kernel::gate::reaches_the_undoable(&reaching) {
                    let verdict = kernel::gate::undoable(&reaching, &self.sandbox, &self.taint);
                    if let Some(answered) = self.settled(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::Egress => {
                // A call that names no destination has no egress to
                // judge; the tool's grammar says which calls name one.
                // A tool that declares Egress and never names a host is
                // a wiring defect, and the bench cannot tell it from a
                // call with nothing to send - so the tool's own tests
                // hold its grammar to naming every destination it has.
                if let GateSubject::Host(host) = subject {
                    let spans = kernel::secret::scan(&scanned(call, "scan egress args")?);
                    let target = kernel::gate::target_of(host);
                    let verdict = kernel::gate::egress(&spans, &target, self.prior_public_egress);
                    if let Some(answered) = self.crossed(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::AttachUserBrowser { address } => {
                // The one door that asks. The address is the tool's
                // registration's, not the call's: every call attaches to
                // the browser the person declared, and a model filling
                // in an address would be inventing a fact the city
                // already holds. `None` is a building that enabled the
                // tool and has not been told where the browser answers,
                // which is the state the person's next action resolves.
                let endpoint = address.as_deref().map(kernel::gate::target_of);
                let verdict = kernel::gate::attach(endpoint.as_ref());
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
                // The bytes are still scanned: what gets typed into a
                // page is the credential this door can see. The page's
                // own host decides the first-public-egress notice, and
                // a call that names no page is the attachment itself,
                // which is this machine.
                let page = match subject {
                    GateSubject::Host(host) => kernel::gate::target_of(host),
                    GateSubject::None => EgressTarget::Loopback,
                    other @ (GateSubject::Area(_)
                    | GateSubject::Room(_)
                    | GateSubject::Scope(_)) => {
                        return Err(subject_not_for(name, other, "drive a browser"));
                    }
                };
                let spans = kernel::secret::scan(&scanned(call, "scan browser args")?);
                let verdict = kernel::gate::egress(&spans, &page, self.prior_public_egress);
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Spawn => {
                // No door here. How deep work may be handed down is
                // a type rather than a question, and the depth a
                // spawn would reach is known where the parent run
                // is: `gate::spawn` runs at the dispatch site, with
                // the parent's `Depth` in hand.
            }
            Effect::Govern => {
                // A run does not rewrite the rules it is judged by. The
                // rules are in `CONFIG.toml` and in the building's own
                // `BUILDING.md`, where a person edits them; a door that
                // asked instead would be a door whose default answer
                // gets clicked through.
                let scope = match subject {
                    GateSubject::Scope(scope) => scope.clone(),
                    GateSubject::None => "the scope this run sits in".to_owned(),
                    other
                    @ (GateSubject::Area(_) | GateSubject::Room(_) | GateSubject::Host(_)) => {
                        return Err(subject_not_for(name, other, "change what governs"));
                    }
                };
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "invoke tool",
                    format!(
                        "`{name}` declares Govern, and a run may not change what governs it \
                         ({scope})"
                    ),
                )
                .with_recovery(
                    "edit the scope's `CONFIG.toml`, or the building's `BUILDING.md`, and                      dispatch again",
                ));
            }
            Effect::Spend => {
                // No Spend tool instance exists until the egress proxy
                // lands (P1); the door is wired so the first one meets it.
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "invoke tool",
                    format!("`{name}` declares Spend, which has no instance before P1"),
                )
                .with_recovery(
                    "do this work with a tool that spends nothing; no tool in this build \
                     can move money",
                ));
            }
        }
        Ok(None)
    }

    /// What one gate's verdict means to this bench.
    ///
    /// Three answers, because a door has three: an Allow lets the call
    /// through, a Deny is the outcome, and an Ask reaches the model as
    /// the door's own question - `E_APPROVAL_PENDING` with the recovery
    /// sentence naming what only the person can do. A pending question
    /// ends the call, not the turn.
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome> {
        match outcome {
            GateOutcome::Allow => None,
            GateOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
            GateOutcome::Ask { question } => Some(BenchOutcome::Refused { refusal: question }),
        }
    }

    /// What one egress verdict means to this bench.
    ///
    /// The first public egress is remembered here, once, for both doors
    /// that can reach outside: a connector's registered destination and
    /// a call's own host.
    fn crossed(&mut self, outcome: EgressOutcome) -> Option<BenchOutcome> {
        match outcome {
            EgressOutcome::Allow {
                first_public_egress,
            } => {
                if first_public_egress {
                    self.prior_public_egress = true;
                }
                None
            }
            EgressOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
        }
    }
}

/// A tool answered with a subject its effect does not read. Refused
/// rather than judged by something else: a tool whose `subject` and
/// `effect` disagree is a wiring defect, and the next reader needs to
/// see it here instead of a gate judging the wrong thing.
fn subject_not_for(name: &str, subject: &GateSubject, doing: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "invoke tool",
        format!(
            "`{name}` declares {doing} and its `subject` answered {}",
            spell(subject)
        ),
    )
    .with_recovery(
        "report this against the tool: `Tool::subject` and `ToolMeta::effect` disagree, \
         and the bench reads the subject the tool answers with",
    )
}

fn spell(subject: &GateSubject) -> &'static str {
    match subject {
        GateSubject::Area(_) => "an area",
        GateSubject::Room(_) => "a room",
        GateSubject::Scope(_) => "a scope",
        GateSubject::Host(_) => "a host",
        GateSubject::None => "no subject",
    }
}

/// One call's arguments as the bytes the secret scan reads.
///
/// Two doors reach outside and both scan the same thing; `doing` names
/// which one, so a failure to serialise says which door it happened at.
///
/// # Errors
/// Refuses arguments that will not serialise, which is a call this
/// bench cannot judge rather than a call it may let through.
fn scanned(call: &ToolCall, doing: &'static str) -> Result<Vec<u8>, AxError> {
    serde_json::to_vec(&call.args).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, doing, err.to_string()).with_recovery(
            "call the tool again with arguments made of strings and whole numbers, \
                 which is all this city's payloads carry",
        )
    })
}
