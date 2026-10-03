// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The tool bench: which door one call goes through, in which order,
//! and what comes back when a door says no.
//!
//! Gate routing is turn-layer work (Handoff verdict 10), so the
//! executor above stays thin: it hands the bench a call and receives an
//! exhaustive [`BenchOutcome`] rather than deciding anything itself.
//!
//! **Three orderings are load-bearing.**
//! - Dedup runs before any side effect, so a replayed call cannot bill
//!   or write twice.
//! - `exec` is forecast for discards *before* the Write door, because
//!   "this command deletes things" is a stronger claim than "this
//!   command writes somewhere" and deserves the stricter door.
//! - A `Deny` comes back as a `tool_result` carrying the refusal rather
//!   than ending the turn: the model that asked for something it may not
//!   have should learn that, and continue.
//!
//! The tool itself arrives as a `Box<dyn Tool>` and the sandbox behind
//! it is a port, so what this module owns is the ordering rather than
//! the effect.

use std::collections::BTreeMap;

use kernel::{
    Address, AxCode, AxError, DiscardForecast, Effect, EgressOutcome, EgressTarget, GateOutcome,
    GateSubject, IdemKey, Locator, TaintSet, Tool, ToolCall, ToolName, ToolOutcome, WriteDomain,
};
use serde_json::Value;
use storage::{Checkpoint, Provenance};

mod admit;
mod outside;

/// The checkpoint net a run works under: the repository, what a checkpoint
/// covers, and who is signing it.
///
/// Three values that are meaningless apart - a checkpoint with no scope
/// stages what the run never held, and a checkpoint with no provenance
/// commits under nobody's name - so they are handed over together
/// rather than assembled inside the bench.
pub struct CheckpointNet {
    pub checkpoint: Checkpoint,
    /// The prefixes a checkpoint stages, which are the run's write domain and
    /// not its room. A checkpoint narrower than the domain leaves whatever
    /// the run wrote in between outside every checkpoint, and therefore
    /// outside `changes` and outside restoration.
    pub scope: Vec<String>,
    pub of: Provenance,
}

/// The tool bench: the turn layer's routing of a call through the gate
/// its own declared Effect names, in the three orderings the module
/// documentation states.
pub struct ToolBench {
    tools: BTreeMap<ToolName, Box<dyn Tool>>,
    domain: WriteDomain,
    taint: TaintSet,
    /// The floor's limits, which decide whether a connector may reach
    /// what nothing here can take back.
    sandbox: kernel::SandboxLimits,
    /// What each key already answered, success or failure. A key alone
    /// would stop the second call's side effect and still owe the model
    /// an answer; the answer is the first call's own, whichever way it
    /// went (`crates/runtime/spec/Bench.lean` §8-35).
    seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>,
    prior_public_egress: bool,
    /// The checkpoint net. A command the forecast suspects of deleting
    /// things does not get refused — text prediction is obfuscatable, so
    /// refusing on a substring would be security theatre that also
    /// blocks honest work. It gets checkpointed instead: commit first, then
    /// run, so whatever it deletes is restorable. Absent a net, such a
    /// command is refused, because running it unprotected is the one
    /// outcome nobody chose.
    net: Option<CheckpointNet>,
    /// What this run was given to do, as the approvals list refers to
    /// it. An item that named no artifact would leave a person deciding
    /// about a spawn with nothing to open.
    job: Option<Locator>,
    /// Where this run works, which is what a delegation approval
    /// clusters by: the person is asked whether this resident may hand
    /// work down, once.
    asking: Option<Address>,
}

/// What the bench decided, alongside what the tool produced.
#[derive(Debug)]
pub enum BenchOutcome {
    /// The tool ran; this is its result. `checkpointed` carries the commit
    /// the wave was checkpointed against when the forecast suspected a
    /// discard, so the post-wave sweep knows what to restore from.
    /// `wrote` is the tool's own account of what the call may have
    /// written, which the next checkpoint stages (`crates/runtime/spec/Run/Checkpoint.lean` §8-45).
    Ran {
        outcome: ToolOutcome,
        checkpointed: Option<String>,
        wrote: kernel::Writes,
    },
    /// A gate refused, or asked. Either way the answer travels back as
    /// a tool_result, which keeps the turn alive and tells the model
    /// what it may not do or what the person has to do; a pending
    /// question carries `E_APPROVAL_PENDING` and its recovery sentence
    /// is the question.
    Refused { refusal: Box<AxError> },
    /// The call was already made, and this is what it answered. A replay
    /// is owed the first result: an error here would teach the model a
    /// call failed when it succeeded.
    Duplicate { outcome: ToolOutcome },
}

/// Each registered tool's answer to what it may write, by declared effect.
pub struct DeclaredWrites(BTreeMap<ToolName, kernel::Writes>);

impl DeclaredWrites {
    /// A name the bench does not hold answers `Domain`: the call will be
    /// refused, and a guess that it writes nothing is the one guess that
    /// could leave a write outside every checkpoint.
    pub fn of(&self, call: &ToolCall) -> kernel::Writes {
        self.0
            .get(&call.name)
            .cloned()
            .unwrap_or(kernel::Writes::Domain)
    }
}

impl ToolBench {
    pub fn new(domain: WriteDomain) -> ToolBench {
        ToolBench {
            tools: BTreeMap::new(),
            domain,
            taint: TaintSet::empty(),
            sandbox: kernel::SandboxLimits::default(),
            seen: BTreeMap::new(),
            prior_public_egress: false,
            net: None,
            job: None,
            asking: None,
        }
    }

    /// Hands the bench the work it serves: where the run stands and what
    /// it was given to do.
    ///
    /// Without it a spawn is refused rather than allowed, because an
    /// approval item that named neither the asker nor an artifact would
    /// reach a person as a question about nothing.
    #[must_use]
    pub fn for_job(mut self, asking: Address, job: Locator) -> ToolBench {
        self.asking = Some(asking);
        self.job = Some(job);
        self
    }

    /// Hands the bench its checkpoint net. Without one, a suspected
    /// discard is refused rather than run unprotected.
    pub fn with_checkpoint(mut self, net: CheckpointNet) -> ToolBench {
        self.net = Some(net);
        self
    }

    /// Registers a tool under its own declared name. A second tool
    /// claiming a taken name is refused rather than shadowing the first.
    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), AxError> {
        let name = tool.meta().name.clone();
        if self.tools.contains_key(&name) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "register tool",
                format!("`{name}` is already registered"),
            )
            .with_recovery(format!(
                "rename one of the two tools declaring the name `{name}` in its \
                 `ToolMeta`, then register it again"
            )));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// What each registered tool may write, read off its declared
    /// effect alone, taken out before the bench is lent to the run so the
    /// checkpoint policy can ask it while a wave waits (`crates/runtime/spec/Run/Checkpoint.lean` §8-45).
    pub fn declared_writes(&self) -> DeclaredWrites {
        DeclaredWrites(
            self.tools
                .iter()
                .map(|(name, tool)| (name.clone(), kernel::Writes::of(&tool.meta().effect)))
                .collect(),
        )
    }

    pub fn taint_mut(&mut self) -> &mut TaintSet {
        &mut self.taint
    }

    /// The registered tool's declaration. Callers packaging a result
    /// need its `temporal` to decide whether a clock line is due.
    pub fn meta_of(&self, name: &ToolName) -> Option<&kernel::ToolMeta> {
        self.tools.get(name).map(|tool| tool.meta())
    }

    /// Routes one call: dedup, then the door its Effect names, then the
    /// tool itself: `clear`, then the tool `tool_for` names, then
    /// `account`, in order, on one thread.
    pub fn invoke(
        &mut self,
        call: &ToolCall,
        key: &IdemKey,
        now: kernel::TimeMs,
    ) -> Result<BenchOutcome, AxError> {
        match self.clear(call, key, now)? {
            Clearance::Answered(outcome) => Ok(outcome),
            Clearance::Cleared(ticket) => {
                let answered = self.tool_for(&ticket).and_then(|tool| tool.invoke(call));
                self.account(ticket, answered)
            }
        }
    }

    /// Dedup, the doors, and the discard forecast's checkpoint: everything
    /// that decides whether a call may run, and reads or writes what
    /// the calls before it left. A concurrent wave clears its calls one
    /// at a time, in call order.
    pub fn clear(
        &mut self,
        call: &ToolCall,
        key: &IdemKey,
        now: kernel::TimeMs,
    ) -> Result<Clearance, AxError> {
        // Before any unreplayable effect (8.2). `seen` holds a key only
        // once the tool answered it, so its keys are the claimed set and
        // a second copy of them would be a second authority.
        if let Some(answered) = self.seen.get(key) {
            return match answered {
                Ok(outcome) => Ok(Clearance::Answered(BenchOutcome::Duplicate {
                    outcome: outcome.clone(),
                })),
                Err(refused) => Err(refused.clone()),
            };
        }
        let name = call.name.as_str();
        let Some(tool) = self.tools.get(&call.name) else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "invoke tool",
                format!("no tool named `{name}` is registered"),
            )
            .with_recovery("call one of the tools listed in your catalog"));
        };
        let effect = tool.effect_of(call)?;
        let wrote = tool.writes(call);
        // What the call is about, in the tool's own grammar.
        // Read before any door: a subject this tool cannot read is a
        // call no door may judge.
        let subject = tool.subject(call)?;

        // The command door stands before the forecast: a command a
        // tainted run may not execute owes no checkpoint.
        if name == ToolName::EXEC
            && let Some(answered) = self.settled(kernel::gate::command(&self.taint))
        {
            return Ok(Clearance::Answered(answered));
        }

        // exec is forecast first. A hit does not refuse: it checkpoints.
        let mut checkpointed = None;
        if name == ToolName::EXEC
            && let Ok(arm) = crate::tools::parse_arm(call.args.as_map())
            && let DiscardForecast::Suspected { pattern } = kernel::discard::forecast(&arm)
        {
            let Some(net) = self.net.as_mut() else {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "invoke tool",
                    format!("`{pattern}` may discard files and no checkpoint net is configured"),
                )
                .with_recovery(
                    "configure the checkpoint net, or run a command that does not delete",
                ));
            };
            let payload = net
                .checkpoint
                .wave_pre(&net.scope, now, &net.of)
                .map_err(kernel_error_from_storage)?;
            checkpointed = payload
                .as_map()
                .get("oid")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }

        if let Some(answered) = self.admit(call, name, &effect, &subject)? {
            return Ok(Clearance::Answered(answered));
        }
        Ok(Clearance::Cleared(Ticket {
            key: *key,
            name: call.name.clone(),
            effect,
            checkpointed,
            wrote,
        }))
    }

    /// The tool a cleared call runs on. What comes back is the tool
    /// rather than the bench, because the bench is not `Sync` (its
    /// checkpoint holds a git repository handle) while a tool is: the
    /// calls of one wave invoke their tools on several threads at once,
    /// and none of them touches the dedup table or the taint.
    ///
    /// # Errors
    /// A ticket naming a tool this bench does not hold.
    pub fn tool_for(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.tool_named(&ticket.name).ok_or_else(|| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "invoke tool",
                format!("no tool named `{}` is registered", ticket.name),
            )
            .with_nearby(self.tools.keys().map(ToString::to_string).collect())
            .with_recovery(
                "call one of the tools listed beside this error; those are the tools \
                 this run holds",
            )
        })
    }

    /// The tool registered under `name`, lent out before any call to it
    /// is cleared, so a read the model hands over can start while it is
    /// still generating (`crates/runtime/spec/Turn.lean` §8-3). A name this bench does not
    /// hold starts nothing early; admitting the call reports it.
    pub fn tool_named(&self, name: &ToolName) -> Option<&dyn Tool> {
        self.tools.get(name).map(AsRef::as_ref)
    }

    /// Records what a cleared call answered. A concurrent wave accounts
    /// its calls in call order, so the dedup table and the taint grow
    /// in the order a serial wave grows them.
    ///
    /// The key is recorded with the answer it earned, so a retry after
    /// a gate refusal is not a replay, and a call the tool itself
    /// failed answers its replay the same way it answered the first
    /// time rather than running again. Outside content enters the run
    /// here and nowhere else, so every later call's doors read the
    /// taint it brought.
    ///
    /// # Errors
    /// The tool's own refusal, and outside content whose source label
    /// is empty.
    pub fn account(
        &mut self,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<BenchOutcome, AxError> {
        let answered = answered.and_then(|outcome| {
            self.taint = outside::entered(&ticket.effect, &self.taint)?;
            Ok(outcome)
        });
        self.seen.insert(ticket.key, answered.clone());
        let outcome = answered?;
        Ok(BenchOutcome::Ran {
            outcome,
            checkpointed: ticket.checkpointed,
            wrote: ticket.wrote,
        })
    }
}

/// What `clear` decided about one call.
#[derive(Debug)]
pub enum Clearance {
    /// The bench answered without running the tool: a replay, or a door
    /// that refused or asked.
    Answered(BenchOutcome),
    /// The call may run; `tool_for` and then `account` take the ticket.
    Cleared(Ticket),
}

/// A call `clear` let through: the tool `tool_for` names, and what
/// `account` records the answer under.
#[derive(Debug)]
pub struct Ticket {
    key: IdemKey,
    name: ToolName,
    effect: Effect,
    /// The commit the discard forecast checkpointed this call against.
    checkpointed: Option<String>,
    /// What the tool says the call may write, asked while the bench
    /// still holds the tool; `account` hands it on in the outcome.
    wrote: kernel::Writes,
}

/// The storage crate owns its own error root; the turn layer speaks
/// AxError, so the conversion happens once, here.
fn kernel_error_from_storage(err: storage::StorageError) -> AxError {
    err.into_ax()
}
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
