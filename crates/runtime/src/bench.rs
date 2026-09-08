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

use std::collections::{BTreeMap, BTreeSet};

use kernel::{
    Address, ApprovalItem, AxCode, AxError, DedupVerdict, DiscardForecast, Effect, EgressOutcome,
    EgressTarget, GateContext, GateOutcome, IdemKey, Locator, TaintSet, Tool, ToolCall,
    ToolOutcome, WriteDomain,
};

use serde_json::Value;

use memory::{Checkpoint, Provenance};

mod admit;

/// The checkpoint net a run works under: the repository, what a fence
/// covers, and who is signing it.
///
/// Three values that are meaningless apart - a fence with no scope
/// stages what the run never held, and a fence with no provenance
/// commits under nobody's name - so they are handed over together
/// rather than assembled inside the bench.
pub struct CheckpointNet {
    pub checkpoint: Checkpoint,
    pub scope: String,
    pub of: Provenance,
}

/// The tool bench: the turn layer's routing of a call through the gate
/// its own declared Effect names (Handoff verdict 10 — gate routing is
/// turn-layer work, so the executor stays thin).
///
/// Three orderings are load-bearing. Dedup runs before any side effect,
/// so a replayed call cannot bill or write twice. `exec` is forecast for
/// discards before the Write door, because "this command deletes things"
/// is a stronger claim than "this command writes somewhere" and deserves
/// the stricter door. And a Deny comes back as a `tool_result` carrying
/// the refusal rather than ending the turn: the model that asked for
/// something it may not have should learn that, and continue.
pub struct ToolBench {
    tools: BTreeMap<String, Box<dyn Tool>>,
    domain: WriteDomain,
    taint: TaintSet,
    seen: BTreeSet<IdemKey>,
    prior_public_egress: bool,
    /// The checkpoint net. A command the forecast suspects of deleting
    /// things does not get refused — text prediction is obfuscatable, so
    /// refusing on a substring would be security theatre that also
    /// blocks honest work. It gets fenced instead: commit first, then
    /// run, so whatever it deletes is restorable. Absent a net, such a
    /// command is refused, because running it unprotected is the one
    /// outcome nobody chose.
    net: Option<CheckpointNet>,
    /// Cluster keys the person has already allowed. Held rather than
    /// looked up: the bench runs inside a drive that owns the ledger,
    /// and a gate that read history mid-wave would be a second reader
    /// of the thing the driver is writing.
    granted: Vec<kernel::ClusterKey>,
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
    /// The tool ran; this is its result. `fenced` carries the commit
    /// the wave was fenced against when the forecast suspected a
    /// discard, so the post-wave sweep knows what to restore from.
    Ran {
        outcome: ToolOutcome,
        fenced: Option<String>,
    },
    /// A gate refused. The refusal travels back as a tool_result, which
    /// keeps the turn alive and tells the model what it may not do.
    Refused { refusal: Box<AxError> },
    /// A gate wants a human. S3 has no answering face, so the caller
    /// sees the pending item's code and the run parks.
    Pending { item: Box<ApprovalItem> },
    /// The call was already made. Its earlier result stands.
    Duplicate,
}

impl ToolBench {
    pub fn new(domain: WriteDomain) -> ToolBench {
        ToolBench {
            tools: BTreeMap::new(),
            domain,
            taint: TaintSet::empty(),
            seen: BTreeSet::new(),
            prior_public_egress: false,
            net: None,
            granted: Vec::new(),
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

    /// Records that this cluster has already been allowed.
    ///
    /// The caller folds these from the ledger's answers, so a resumed
    /// run does not stop at the door the person just opened.
    pub fn grant(&mut self, cluster: kernel::ClusterKey) {
        self.granted.push(cluster);
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
        let name = tool.meta().name.as_str().to_owned();
        if self.tools.contains_key(&name) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "register tool",
                format!("`{name}` is already registered"),
            ));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    pub fn taint_mut(&mut self) -> &mut TaintSet {
        &mut self.taint
    }

    /// The registered tool's declaration. Callers packaging a result
    /// need its `temporal` to decide whether a clock line is due.
    pub fn meta_of(&self, name: &str) -> Option<&kernel::ToolMeta> {
        self.tools.get(name).map(|tool| tool.meta())
    }

    /// Routes one call: dedup, then the door its Effect names, then the
    /// tool itself.
    pub fn invoke(
        &mut self,
        call: &ToolCall,
        key: &IdemKey,
        ctx: &GateContext,
    ) -> Result<BenchOutcome, AxError> {
        // Before any unreplayable effect (8.2).
        if kernel::dedup(&self.seen, key) == DedupVerdict::Duplicate {
            return Ok(BenchOutcome::Duplicate);
        }
        let name = call.name.as_str().to_owned();
        let Some(tool) = self.tools.get(&name) else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "invoke tool",
                format!("no tool named `{name}` is registered"),
            )
            .with_recovery("call one of the tools listed in your catalog"));
        };
        let effect = tool.meta().effect.clone();

        // exec is forecast first. A hit does not refuse: it fences.
        let mut fenced = None;
        if name == "exec"
            && let Ok(arm) = crate::tools::parse_arm(call.args.as_map())
            && let DiscardForecast::Suspected { pattern } = kernel::forecast(&arm)
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
                .wave_pre(&net.scope, ctx.now, &net.of)
                .map_err(kernel_error_from_memory)?;
            fenced = payload
                .as_map()
                .get("oid")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }

        if let Some(answered) = self.admit(call, &name, &effect, ctx)? {
            return Ok(answered);
        }

        // The key is recorded once the call is committed to, so a retry
        // after a gate refusal is not treated as a replay.
        self.seen.insert(*key);
        // Re-borrowed here: the forecast fence needed `self` mutably.
        let Some(tool) = self.tools.get_mut(&name) else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "invoke tool",
                format!("no tool named `{name}` is registered"),
            ));
        };
        let outcome = tool.invoke(call)?;
        Ok(BenchOutcome::Ran { outcome, fenced })
    }
}

/// The memory crate owns its own error root; the turn layer speaks
/// AxError, so the conversion happens once, here.
fn kernel_error_from_memory(err: memory::MemoryError) -> AxError {
    err.into_ax()
}
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
