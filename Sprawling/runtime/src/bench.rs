// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{
    Address, AxCode, AxError, DiscardForecast, Effect, EgressOutcome, EgressTarget, GateOutcome,
    GateSubject, IdemKey, Locator, TaintSet, Tool, ToolCall, ToolName, ToolOutcome, WriteDomain,
};
use serde_json::Value;
use storage::{Checkpoint, Provenance};

mod admit;
mod outside;

pub struct CheckpointNet {
    pub checkpoint: Checkpoint,

    pub scope: Vec<String>,
    pub of: Provenance,
}

pub struct ToolBench {
    tools: BTreeMap<ToolName, Box<dyn Tool>>,
    domain: WriteDomain,
    taint: TaintSet,

    sandbox: kernel::SandboxLimits,

    seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>,
    prior_public_egress: bool,

    egress: Option<kernel::EgressAllowlist>,

    net: Option<CheckpointNet>,

    job: Option<Locator>,

    asking: Option<Address>,
}

#[derive(Debug)]
pub enum BenchOutcome {

    Ran {
        outcome: ToolOutcome,
        checkpointed: Option<String>,
        wrote: kernel::Writes,
    },

    Refused { refusal: Box<AxError> },

    Duplicate { outcome: ToolOutcome },
}

pub struct DeclaredWrites(BTreeMap<ToolName, kernel::Writes>);

impl DeclaredWrites {

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
            egress: None,
            net: None,
            job: None,
            asking: None,
        }
    }

    #[must_use]
    pub fn for_job(mut self, asking: Address, job: Locator) -> ToolBench {
        self.asking = Some(asking);
        self.job = Some(job);
        self
    }

    pub fn with_checkpoint(mut self, net: CheckpointNet) -> ToolBench {
        self.net = Some(net);
        self
    }

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

    pub fn meta_of(&self, name: &ToolName) -> Option<&kernel::ToolMeta> {
        self.tools.get(name).map(|tool| tool.meta())
    }

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

    pub fn clear(
        &mut self,
        call: &ToolCall,
        key: &IdemKey,
        now: kernel::TimeMs,
    ) -> Result<Clearance, AxError> {

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

        let subject = tool.subject(call)?;

        if name == ToolName::EXEC
            && let Some(answered) = self.settled(kernel::gate::command(&self.taint))
        {
            return Ok(Clearance::Answered(answered));
        }

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
                .map_err(storage::StorageError::into_ax)?;
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

    pub fn tool_named(&self, name: &ToolName) -> Option<&dyn Tool> {
        self.tools.get(name).map(AsRef::as_ref)
    }

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

#[derive(Debug)]
pub enum Clearance {

    Answered(BenchOutcome),

    Cleared(Ticket),
}

#[derive(Debug)]
pub struct Ticket {
    key: IdemKey,
    name: ToolName,
    effect: Effect,

    checkpointed: Option<String>,

    wrote: kernel::Writes,
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
