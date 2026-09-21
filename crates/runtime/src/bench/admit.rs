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
    /// tool result rather than ending the turn, and an escalation parks
    /// unless the person has already allowed that cluster.
    ///
    /// # Errors
    /// Refuses a call that declares an effect this bench does not route,
    /// an egress that names no host, a spawn or a rule change on a bench
    /// built without a job, and arguments that will not serialise for
    /// the secret scan.
    pub(super) fn admit(
        &mut self,
        call: &ToolCall,
        name: &str,
        effect: &Effect,
    ) -> Result<Option<BenchOutcome>, AxError> {
        match effect {
            Effect::Read => {}
            // The tool declares an area, not a file; the file is judged
            // by the tool itself once a call names one.
            Effect::Write { domain: area } => {
                let verdict = kernel::gate::reach(&self.domain, area, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
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
                let spans = kernel::secret::scan(&scanned(call, "scan egress args")?);
                // The target is the tool's to declare; a call that does
                // not say where it is sending cannot be judged, and an
                // unjudged egress is the one thing the door exists for.
                let host = call
                    .args
                    .as_map()
                    .get("host")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AxError::failure(
                            AxCode::InvalidArgs,
                            "invoke tool",
                            format!("`{name}` declares Egress but named no host"),
                        )
                        .with_recovery(
                            "call the tool again with a `host` argument naming the \
                             domain it reaches",
                        )
                    })?
                    .to_owned();
                let verdict = kernel::gate::egress(
                    &spans,
                    &EgressTarget::Public { host },
                    self.prior_public_egress,
                );
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
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "invoke tool",
                    format!("`{name}` declares Govern, and a run may not change what governs it"),
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
    /// What one gate verdict means to this bench.
    ///
    /// Two answers, because a door has two: an Allow lets the call
    /// through and a Deny is the outcome. No door asks a person, so
    /// there is no third state for this to carry.
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome> {
        match outcome {
            GateOutcome::Allow => None,
            GateOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
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
