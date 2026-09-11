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
        ctx: &GateContext,
    ) -> Result<Option<BenchOutcome>, AxError> {
        match effect {
            Effect::Read => {}
            // The tool declares an area, not a file; the file is judged
            // by the tool itself once a call names one.
            Effect::Write { domain: area } => {
                let verdict = kernel::reach(&self.domain, area, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Connector { label } => {
                // Same door, same scan; only the target differs. A
                // connector's destination is its registration's, so
                // there is nothing for the call to name and nothing for
                // a model to get wrong.
                let spans = kernel::scan(&scanned(call, "scan connector args")?);
                let verdict = kernel::egress(
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
                // the answer is no — a key pressed on somebody's own
                // machine has no restoration — so it goes to the person
                // rather than being refused, which would make the tool
                // equivalent to absent.
                let called = ToolName::parse(name)?;
                let reaching = kernel::ConnectorCall {
                    label,
                    tool: &called,
                };
                if kernel::reaches_the_undoable(&reaching) {
                    let Some(job) = self.job.as_ref() else {
                        return Err(AxError::failure(
                            AxCode::ToolUnavailable,
                            "invoke tool",
                            format!(
                                "`{name}` reaches this machine's desktop and this bench was \
                                 built without a job"
                            ),
                        )
                        .with_recovery(
                            "build the bench with `for_job`; an action on somebody's own \
                             machine that they cannot be asked about is one nobody allowed",
                        ));
                    };
                    let verdict = kernel::undoable(ctx, &reaching, job, &self.taint);
                    if let Some(answered) = self.settled(verdict) {
                        return Ok(Some(answered));
                    }
                }
            }
            Effect::Egress => {
                let spans = kernel::scan(&scanned(call, "scan egress args")?);
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
                    })?
                    .to_owned();
                let verdict = kernel::egress(
                    &spans,
                    &EgressTarget::Public { host },
                    self.prior_public_egress,
                );
                if let Some(answered) = self.crossed(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Spawn => {
                let (Some(asking), Some(job)) = (self.asking.as_ref(), self.job.as_ref()) else {
                    return Err(AxError::failure(
                        AxCode::ToolUnavailable,
                        "invoke tool",
                        format!("`{name}` declares Spawn and this bench was built without a job"),
                    )
                    .with_recovery(
                        "build the bench with `for_job`; a spawn a person cannot be asked about \
                         is a spawn nobody allowed",
                    ));
                };
                // The room is the tool's own argument, so the person is
                // told where the work is going without this layer
                // learning the tool's schema: an unreadable room reads
                // as the asking address, and the item still names a real
                // place.
                let room = call
                    .args
                    .as_map()
                    .get("room")
                    .and_then(Value::as_str)
                    .and_then(|raw| Address::parse(raw).ok())
                    .unwrap_or_else(|| asking.clone());
                let verdict = kernel::delegation(ctx, asking, &room, job, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Govern => {
                let (Some(asking), Some(job)) = (self.asking.as_ref(), self.job.as_ref()) else {
                    return Err(AxError::failure(
                        AxCode::ToolUnavailable,
                        "invoke tool",
                        format!("`{name}` declares Govern and this bench was built without a job"),
                    )
                    .with_recovery(
                        "build the bench with `for_job`; a rule change a person cannot be asked \
                         about is a rule change nobody allowed",
                    ));
                };
                let scope = call
                    .args
                    .as_map()
                    .get("scope")
                    .and_then(Value::as_str)
                    .and_then(|raw| Address::parse(raw).ok())
                    .unwrap_or_else(|| asking.clone());
                // What the person is being asked to allow, in their own
                // reading rather than as a category name.
                let proposal = call
                    .args
                    .as_map()
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let verdict = kernel::govern(ctx, asking, &scope, proposal, job, &self.taint);
                if let Some(answered) = self.settled(verdict) {
                    return Ok(Some(answered));
                }
            }
            Effect::Spend => {
                // No Spend tool instance exists until the egress proxy
                // lands (P1); the door is wired so the first one meets it.
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "invoke tool",
                    format!("`{name}` declares Spend, which has no instance before P1"),
                ));
            }
            _ => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "invoke tool",
                    format!("`{name}` declares an effect this bench does not route"),
                ));
            }
        }
        Ok(None)
    }

    /// What one gate's verdict means to this bench.
    ///
    /// The granted check lives here and nowhere else. An answer the
    /// person already gave is not asked again, and the grant is per
    /// cluster because that is the unit they were shown and answered
    /// in. The rule used to be written out at each of the three doors
    /// that can escalate.
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome> {
        match outcome {
            GateOutcome::Allow => None,
            GateOutcome::Deny { refusal } => Some(BenchOutcome::Refused { refusal }),
            GateOutcome::Escalate { item } => {
                (!self.granted.contains(&item.cluster_key)).then(|| BenchOutcome::Pending {
                    item: Box::new(item),
                })
            }
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
    serde_json::to_vec(&call.args)
        .map_err(|err| AxError::failure(AxCode::InvalidArgs, doing, err.to_string()))
}
